// The differential oracle, level 2 (docs/drahtseilakt/scripts.md, "C oracle, level 2"): one synthetic graph per block
// kind, its pins swept over edge values (zero and negative times and counts, Reset during a run, re-entry in the same
// frame, the script deactivated mid-run), run in our runtime and in the upstream C runtime (tests/oracle/ck_oracle.c,
// a test-only reference built with cmake). The traces must be equal frame by frame: the blocks run, in order, the
// scripts and blocks active after the frame and every output parameter of the block under test. The C code is itself
// a reimplementation: a disagreement is settled by the notes (docs/), and only where they are silent by the C, with
// the finding written down. A case whose loop spins into the iteration cap is not compared: docs/ck-runtime.md says
// at most BehaviorMaxIteration pops a frame, the C makes one more. Skipped unless OPENBALLANCE_ORACLE names the built
// ck_oracle.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::type_complexity
)]

use script::blocks::proto;
use script::exec::{Ent, Host};
use script::{
    Array, Block, Graph, Guid, Id, Io, Link, Node, ObjInfo, ObjKind, Param, Runtime, Value, types,
};
use std::fmt::Write as _;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Pin,
    Pout,
    Local,
}

/// A block of the description: any prototype, its inputs and outputs by count, its parameters in order.
#[derive(Clone, Debug)]
struct B {
    script: usize,
    guid: Guid,
    prio: i32,
    nin: usize,
    nout: usize,
    /// (kind, type as the oracle names it, value)
    params: Vec<(Kind, &'static str, String)>,
    /// the target parameter (type, value)
    target: Option<(&'static str, String)>,
}

impl B {
    fn new(script: usize, guid: Guid, nin: usize, nout: usize) -> B {
        B {
            script,
            guid,
            prio: 0,
            nin,
            nout,
            params: Vec::new(),
            target: None,
        }
    }
    fn pin(mut self, t: &'static str, v: impl ToString) -> B {
        self.params.push((Kind::Pin, t, v.to_string()));
        self
    }
    fn pout(mut self, t: &'static str, v: impl ToString) -> B {
        self.params.push((Kind::Pout, t, v.to_string()));
        self
    }
    fn target(mut self, t: &'static str, v: impl ToString) -> B {
        self.target = Some((t, v.to_string()));
        self
    }
    fn local(mut self, t: &'static str, v: impl ToString) -> B {
        self.params.push((Kind::Local, t, v.to_string()));
        self
    }
}

/// A graph description, the text both sides read (the format: tests/oracle/ck_oracle.c).
#[derive(Default, Clone, Debug)]
struct Spec {
    objects: usize,
    /// (object, active at the start)
    scripts: Vec<(usize, bool)>,
    /// members by object index
    groups: Vec<Vec<usize>>,
    /// arrays: (columns, rows, key column or -1, the kind of each column as a letter: i f s o, the missing ones
    /// i, modulus). The cell (r, c) is 10 r + c, or (7 r + 3 c) % modulus when that is not 0; a float is half of
    /// it, a string "s" and it, an object the one it names (modulo the objects).
    arrays: Vec<(usize, usize, i32, &'static str, usize)>,
    blocks: Vec<B>,
    /// (script, from: None = the start, Some((block, output)), (to block, input), delay)
    links: Vec<(usize, Option<(usize, usize)>, (usize, usize), u16)>,
    frames: u32,
}

impl Spec {
    fn add(&mut self, b: B) -> usize {
        self.blocks.push(b);
        self.blocks.len() - 1
    }

    fn link(&mut self, from: Option<(usize, usize)>, to: (usize, usize), delay: u16) {
        let s = self.blocks[to.0].script;
        self.links.push((s, from, to, delay));
    }

    fn text(&self) -> String {
        let mut t = String::new();
        for o in 0..self.objects {
            writeln!(t, "object O{o}").unwrap();
        }
        for (i, (o, on)) in self.scripts.iter().enumerate() {
            writeln!(t, "script S{i} {o} {}", u8::from(*on)).unwrap();
        }
        for (i, m) in self.groups.iter().enumerate() {
            let m: Vec<String> = m.iter().map(|x| x.to_string()).collect();
            writeln!(t, "group G{i} {}", m.join(" ")).unwrap();
        }
        for (i, (c, r, k, kinds, m)) in self.arrays.iter().enumerate() {
            let kinds = if kinds.is_empty() { "-" } else { kinds };
            writeln!(t, "array A{i} {c} {r} {k} {kinds} {m}").unwrap();
        }
        for (i, b) in self.blocks.iter().enumerate() {
            writeln!(
                t,
                "bb B{i} {} {:08x}:{:08x} {} {} {}",
                b.script, b.guid.0, b.guid.1, b.prio, b.nin, b.nout
            )
            .unwrap();
            for (k, ty, v) in &b.params {
                let k = match k {
                    Kind::Pin => "pin",
                    Kind::Pout => "pout",
                    Kind::Local => "local",
                };
                writeln!(t, "{k} {i} {ty} {v}").unwrap();
            }
            if let Some((ty, v)) = &b.target {
                writeln!(t, "target {i} {ty} {v}").unwrap();
            }
        }
        for (s, from, (to, inp), d) in &self.links {
            let from = from.map_or("s".to_string(), |(b, o)| format!("{b}.{o}"));
            writeln!(t, "link {s} {from} {to}.{inp} {d}").unwrap();
        }
        writeln!(t, "frames {}", self.frames).unwrap();
        t
    }

    /// The same graph for our runtime: (graph, scripts, blocks, the printed output parameters (block, index, id, type)).
    fn graph(
        &self,
    ) -> (
        Graph,
        Vec<Id>,
        Vec<Id>,
        Vec<(usize, usize, Id, &'static str)>,
    ) {
        let mut objects: Vec<ObjInfo> = Vec::new();
        let mut nodes: Vec<(Id, Node)> = Vec::new();
        let add = |objects: &mut Vec<ObjInfo>, name: String, kind: ObjKind| -> Id {
            // the classes the oracle creates its objects with (behavioural objects, behaviours, groups, arrays)
            let class = match kind {
                ObjKind::Entity(_) => 19,
                ObjKind::Behavior => 8,
                ObjKind::Group(_) => 23,
                ObjKind::Array(_) => 52,
                _ => 0,
            };
            objects.push(ObjInfo { name, class, kind });
            objects.len() - 1
        };
        let obj_ids: Vec<Id> = (0..self.objects)
            .map(|o| add(&mut objects, format!("O{o}"), ObjKind::Entity(o)))
            .collect();
        let script_ids: Vec<Id> = (0..self.scripts.len())
            .map(|i| add(&mut objects, format!("S{i}"), ObjKind::Behavior))
            .collect();
        let group_ids: Vec<Id> = (0..self.groups.len())
            .map(|i| add(&mut objects, format!("G{i}"), ObjKind::Group(i)))
            .collect();
        let array_ids: Vec<Id> = (0..self.arrays.len())
            .map(|i| add(&mut objects, format!("A{i}"), ObjKind::Array(i)))
            .collect();
        let script_in: Vec<Id> = script_ids
            .iter()
            .map(|_| {
                let id = add(&mut objects, "In".into(), ObjKind::Other);
                nodes.push((id, Node::Io(Io::default())));
                id
            })
            .collect();
        let mut subs = vec![Vec::new(); self.scripts.len()];
        let mut links = vec![Vec::new(); self.scripts.len()];
        let mut block_ids = Vec::new();
        let mut ios: Vec<(Vec<Id>, Vec<Id>)> = Vec::new();
        let mut printed = Vec::new();
        let value = |ty: &str, v: &str| -> (Guid, Value) {
            let idx = |of: &[Id]| {
                v.parse::<i64>()
                    .ok()
                    .and_then(|i| usize::try_from(i).ok())
                    .and_then(|i| of.get(i).copied())
            };
            match ty {
                "bool" => (types::BOOL, Value::Bool(v.parse::<i32>().unwrap() != 0)),
                "int" => (types::INT, Value::Int(v.parse().unwrap())),
                "float" => (types::FLOAT, Value::Float(v.parse().unwrap())),
                "time" => (types::TIME, Value::Float(v.parse().unwrap())),
                "string" => (types::STRING, Value::Str(v.into())),
                "msg" => (types::MESSAGE, Value::Str(v.into())),
                "obj" => (types::BEOBJECT, Value::Obj(idx(&obj_ids))),
                "script" => (types::SCRIPT, Value::Obj(idx(&script_ids))),
                "group" => (types::GROUP, Value::Obj(idx(&group_ids))),
                "array" => (types::ARRAY, Value::Obj(idx(&array_ids))),
                "percentage" => (types::PERCENTAGE, Value::Float(v.parse().unwrap())),
                "classid" => (types::CLASS_ID, Value::Int(v.parse().unwrap())),
                "vector" | "vec2" | "box" => {
                    let f: Vec<f32> = v.split(',').map(|x| x.parse().unwrap()).collect();
                    let at = |k: usize| f.get(k).copied().unwrap_or(0.0);
                    match ty {
                        "vector" => (types::VECTOR, Value::Vec3([at(0), at(1), at(2)])),
                        "vec2" => (types::VECTOR2, Value::Vec2([at(0), at(1)])),
                        _ => (types::BOX, Value::Box(std::array::from_fn(at))),
                    }
                }
                _ => panic!("type {ty}"),
            }
        };
        for (i, b) in self.blocks.iter().enumerate() {
            let id = add(&mut objects, format!("B{i}"), ObjKind::Behavior);
            let io =
                |n: usize, objects: &mut Vec<ObjInfo>, nodes: &mut Vec<(Id, Node)>| -> Vec<Id> {
                    (0..n)
                        .map(|_| {
                            let x = add(objects, "io".into(), ObjKind::Other);
                            nodes.push((x, Node::Io(Io::default())));
                            x
                        })
                        .collect()
                };
            let ins = io(b.nin, &mut objects, &mut nodes);
            let outs = io(b.nout, &mut objects, &mut nodes);
            let (mut pins, mut pouts, mut locals) = (Vec::new(), Vec::new(), Vec::new());
            for (k, ty, v) in &b.params {
                match k {
                    Kind::Pin if *ty == "none" => {
                        let p = add(&mut objects, "pin".into(), ObjKind::Other);
                        nodes.push((
                            p,
                            Node::Param(Param {
                                name: "pin".into(),
                                guid: types::INT,
                                ..Param::default()
                            }),
                        ));
                        pins.push(p);
                    }
                    Kind::Pin => {
                        let (g, val) = value(ty, v);
                        let src = add(&mut objects, "value".into(), ObjKind::Other);
                        nodes.push((
                            src,
                            Node::Param(Param {
                                name: "value".into(),
                                guid: g,
                                value: val,
                                ..Param::default()
                            }),
                        ));
                        let p = add(&mut objects, "pin".into(), ObjKind::Other);
                        nodes.push((
                            p,
                            Node::Param(Param {
                                name: "pin".into(),
                                guid: g,
                                source: Some(src),
                                ..Param::default()
                            }),
                        ));
                        pins.push(p);
                    }
                    Kind::Pout | Kind::Local => {
                        let (g, val) = value(ty, v);
                        let name = if *k == Kind::Pout { "pout" } else { "local" };
                        let p = add(&mut objects, name.into(), ObjKind::Other);
                        nodes.push((
                            p,
                            Node::Param(Param {
                                name: name.into(),
                                guid: g,
                                value: val,
                                ..Param::default()
                            }),
                        ));
                        if *k == Kind::Pout {
                            printed.push((i, pouts.len(), p, *ty));
                            pouts.push(p);
                        } else {
                            locals.push(p);
                        }
                    }
                }
            }
            let target = b.target.as_ref().map(|(ty, v)| {
                let (g, val) = value(ty, v);
                let src = add(&mut objects, "value".into(), ObjKind::Other);
                nodes.push((
                    src,
                    Node::Param(Param {
                        name: "value".into(),
                        guid: g,
                        value: val,
                        ..Param::default()
                    }),
                ));
                let p = add(&mut objects, "target".into(), ObjKind::Other);
                nodes.push((
                    p,
                    Node::Param(Param {
                        name: "target".into(),
                        guid: g,
                        source: Some(src),
                        ..Param::default()
                    }),
                ));
                p
            });
            nodes.push((
                id,
                Node::Block(Block {
                    name: format!("B{i}"),
                    target,
                    proto: Some(b.guid),
                    priority: b.prio,
                    inputs: ins.clone(),
                    outputs: outs.clone(),
                    pins,
                    pouts,
                    locals,
                    ..Block::default()
                }),
            ));
            subs[b.script].push(id);
            block_ids.push(id);
            ios.push((ins, outs));
        }
        for &(s, from, (to, inp), delay) in &self.links {
            let src = match from {
                None => script_in[s],
                Some((b, o)) => ios[b].1[o],
            };
            let l = add(&mut objects, "link".into(), ObjKind::Other);
            nodes.push((
                l,
                Node::Link(Link {
                    delay,
                    src,
                    dst: ios[to].0[inp],
                }),
            ));
            links[s].push(l);
        }
        for (i, &id) in script_ids.iter().enumerate() {
            nodes.push((
                id,
                Node::Block(Block {
                    name: format!("S{i}"),
                    inputs: vec![script_in[i]],
                    sub: std::mem::take(&mut subs[i]),
                    links: std::mem::take(&mut links[i]),
                    flags: 0x2,
                    ..Block::default()
                }),
            ));
        }
        let scripts = script_ids
            .iter()
            .zip(&self.scripts)
            .map(|(&id, &(o, _))| (id, Some(obj_ids[o])))
            .collect();
        let arrays = self
            .arrays
            .iter()
            .enumerate()
            .map(|(i, &(c, r, k, kinds, m))| {
                let kind_of = |col: usize| match kinds.chars().nth(col) {
                    Some('f') => 2,
                    Some('s') => 3,
                    Some('o') => 4,
                    _ => 1,
                };
                Array {
                    name: format!("A{i}"),
                    columns: (0..c).map(|col| ("c".to_string(), kind_of(col))).collect(),
                    rows: (0..r)
                        .map(|row| {
                            (0..c)
                                .map(|col| {
                                    let v = if m > 0 {
                                        (7 * row + 3 * col) % m
                                    } else {
                                        10 * row + col
                                    };
                                    match kind_of(col) {
                                        2 => Value::Float(v as f32 / 2.0),
                                        3 => Value::Str(format!("s{v}")),
                                        4 => Value::Obj(
                                            obj_ids.get(v % self.objects.max(1)).copied(),
                                        ),
                                        _ => Value::Int(v as i32),
                                    }
                                })
                                .collect()
                        })
                        .collect(),
                    key: usize::try_from(k).ok(),
                }
            })
            .collect();
        let mut g = Graph::assemble(objects, nodes, arrays, scripts).expect("assembles");
        g.start_off = script_ids
            .iter()
            .zip(&self.scripts)
            .filter(|(_, (_, on))| !on)
            .map(|(&id, _)| id)
            .collect();
        (g, script_ids, block_ids, printed)
    }

    /// Our trace in the oracle's form.
    fn run(&self) -> String {
        let (g, scripts, blocks, printed) = self.graph();
        let mut rt = Runtime::new(Arc::new(g));
        rt.set_trace(true);
        let mut host = TestHost {
            groups: self.groups.clone(),
        };
        let mut out = String::new();
        for f in 1..=self.frames {
            writeln!(out, "frame {f}").unwrap();
            rt.tick(&mut host);
            for l in rt.take_trace() {
                if let Some(rest) = l.strip_prefix("run ") {
                    let name = rest.split_once(' ').map_or(rest, |(_, n)| n);
                    writeln!(out, "run {name}").unwrap();
                }
            }
            out.push_str("end");
            for &s in scripts.iter().chain(&blocks) {
                if rt.is_active(s) {
                    write!(out, " {}", rt.graph.name(s)).unwrap();
                }
            }
            out.push('\n');
            for &(b, k, p, ty) in &printed {
                let v = rt.values.get(p).cloned().unwrap_or_default();
                let shown = match ty {
                    "float" | "time" | "percentage" => format!("{:08x}", v.as_float().to_bits()),
                    "vector" | "vec2" | "box" => {
                        let c: Vec<f32> = match &v {
                            Value::Vec3(a) => a.to_vec(),
                            Value::Vec2(a) => a.to_vec(),
                            Value::Box(a) => a.to_vec(),
                            _ => vec![0.0; 6],
                        };
                        let n = match ty {
                            "vector" => 3,
                            "vec2" => 2,
                            _ => 6,
                        };
                        (0..n)
                            .map(|k| format!("{:08x}", c.get(k).copied().unwrap_or(0.0).to_bits()))
                            .collect::<Vec<_>>()
                            .join(",")
                    }
                    "string" => v.as_string(),
                    "obj" | "script" | "group" | "array" => v
                        .as_obj()
                        .map_or("-".to_string(), |o| rt.graph.name(o).to_string()),
                    _ => v.as_int().to_string(),
                };
                writeln!(out, "pout B{b} {k} {shown}").unwrap();
            }
            for (i, a) in rt.arrays().iter().enumerate() {
                let kinds: Vec<String> = a.columns.iter().map(|c| c.1.to_string()).collect();
                let kinds: String = kinds.iter().map(|k| format!(" {k}")).collect();
                writeln!(
                    out,
                    "array A{i} key {} cols{kinds}",
                    a.key.map_or(-1, |k| k as i32)
                )
                .unwrap();
                for row in &a.rows {
                    out.push_str("row");
                    for (c, v) in a.columns.iter().zip(row) {
                        match c.1 {
                            2 => write!(out, " {:08x}", v.as_float().to_bits()),
                            3 => write!(out, " \"{}\"", v.as_string()),
                            4 => write!(
                                out,
                                " {}",
                                v.as_obj()
                                    .map(|o| rt.graph.name(o).to_string())
                                    .filter(|n| !n.is_empty())
                                    .unwrap_or("-".to_string())
                            ),
                            _ => write!(out, " {}", v.as_int()),
                        }
                        .unwrap();
                    }
                    out.push('\n');
                }
            }
        }
        out
    }
}

/// The engine as the oracle has it: a frame of 1000/60 ms in single precision, groups of objects, nothing else.
struct TestHost {
    groups: Vec<Vec<usize>>,
}

impl Host for TestHost {
    fn delta_ms(&self) -> f32 {
        1000.0f32 / 60.0f32
    }
    fn matrix(&self, _: Ent) -> Option<[[f32; 4]; 4]> {
        None
    }
    fn initial_matrix(&self, _: Ent) -> Option<[[f32; 4]; 4]> {
        None
    }
    fn set_matrix(&mut self, _: Ent, _: [[f32; 4]; 4]) {}
    fn set_visible(&mut self, _: Ent, _: bool) {}
    fn bounds(&self, _: Ent) -> Option<([f32; 3], [f32; 3])> {
        None
    }
    fn delete(&mut self, _: Ent) {}
    fn physicalize(&mut self, _: Ent, _: bool) {}
    fn particles(&mut self, _: Ent, _: bool) {}
    fn mesh_of(&self, _: Ent) -> Option<usize> {
        None
    }
    fn group_members(&self, g: usize) -> Vec<usize> {
        self.groups.get(g).cloned().unwrap_or_default()
    }
    fn game_cell(&self, _: &str, _: usize, _: usize) -> Option<Value> {
        None
    }
    fn message(&mut self, _: &str, _: Option<Ent>) {}

    fn note(&mut self, _: &str) {}
}

/// A small generator (xorshift), so a failing case is found again by its seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }
    fn below(&mut self, n: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x % n.max(1) as u64) as usize
    }
    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(of.len())]
    }
}

/// The frame of every case: object O0 with script S0 holding the block under test (block 0), a Nop probe on each of
/// its outputs (so the trace shows which output fired when), the block's inputs fired from the script's start at
/// random delays, some outputs looped back into inputs, and a driver script S1 on O1 that resets, deactivates and
/// reactivates S0 at random frames.
fn frame(r: &mut Rng, block: B, loops: &[(usize, usize, &[u16])]) -> Spec {
    let mut s = Spec {
        objects: 3,
        scripts: vec![(0, true), (1, true)],
        groups: vec![vec![0, 1, 2], vec![]],
        frames: 40,
        ..Spec::default()
    };
    let (nin, nout) = (block.nin, block.nout);
    let x = s.add(block);
    for o in 0..nout {
        let p = s.add(B::new(0, proto::NOP, 1, 1));
        s.link(Some((x, o)), (p, 0), 0);
    }
    // inputs from the start: input 0 always, the others sometimes, at edge delays (0 = the first frame)
    for i in 0..nin {
        for _ in 0..if i == 0 { 1 + r.below(2) } else { r.below(3) } {
            let d = r.pick(&[0u16, 0, 1, 2, 3, 5, 8, 13]);
            s.link(None, (x, i), d);
        }
    }
    for &(o, i, delays) in loops {
        if r.below(3) != 0 {
            let d = r.pick(delays);
            s.link(Some((x, o)), (x, i), d);
        }
    }
    // the driver: Activate Script (Reset? random) and Deactivate Script on S0 at random frames
    for _ in 0..r.below(4) {
        let n = s.add(B::new(1, proto::NOP, 1, 1));
        let d = 1 + r.below(25) as u16;
        s.link(None, (n, 0), d);
        let a = if r.below(2) == 0 {
            s.add(
                B::new(1, proto::ACTIVATE_SCRIPT, 1, 1)
                    .pin("bool", r.below(2))
                    .pin("script", 0),
            )
        } else {
            s.add(B::new(1, proto::DEACTIVATE_SCRIPT, 1, 1).pin("script", 0))
        };
        s.link(Some((n, 0)), (a, 0), 0);
    }
    s
}

fn oracle(bin: &std::path::Path, file: &std::path::Path, spec: &Spec) -> String {
    std::fs::write(file, spec.text()).unwrap();
    let out = std::process::Command::new(bin)
        .arg(file)
        .output()
        .expect("the oracle runs");
    assert!(
        out.status.success(),
        "the oracle failed ({:?}) on:\n{}",
        out.status,
        spec.text()
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Whether a frame of a trace ran the iteration cap (a loop with no way out within the frame).
fn hits_the_cap(trace: &str) -> bool {
    trace
        .split("frame ")
        .any(|f| f.matches("\nrun ").count() >= 7_000)
}

/// Every case of one kind: ours against the C, frame by frame.
fn sweep(kind: &str, make: impl Fn(&mut Rng) -> Spec) {
    let Some(bin) = std::env::var_os("OPENBALLANCE_ORACLE") else {
        // our side alone still runs, deterministically
        for seed in 0..20 {
            let spec = make(&mut Rng::new(seed));
            assert_eq!(spec.run(), spec.run(), "{kind} seed {seed}");
        }
        return;
    };
    let bin = std::path::PathBuf::from(bin);
    let cases: u64 = std::env::var("ORACLE_CASES")
        .ok()
        .and_then(|c| c.parse().ok())
        .unwrap_or(300);
    let dir = std::env::temp_dir().join(format!(
        "drahtseilakt-oracle2-{}-{}",
        kind.replace(' ', "_"),
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("graph.txt");
    for seed in 0..cases {
        let spec = make(&mut Rng::new(seed));
        let ours = spec.run();
        let theirs = oracle(&bin, &file, &spec);
        if hits_the_cap(&theirs) {
            // a loop spinning into BehaviorMaxIteration: the C pops once more than the notes say (see the top)
            continue;
        }
        if ours != theirs {
            let at = ours
                .lines()
                .zip(theirs.lines())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            let window = |t: &str| -> String {
                t.lines()
                    .skip(at.saturating_sub(12))
                    .take(16)
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            panic!(
                "{kind} seed {seed}: traces differ at line {at}\n--- graph\n{}--- ours\n{}\n--- C\n{}",
                spec.text(),
                window(&ours),
                window(&theirs)
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

const TIMES: &[&str] = &[
    "-20", "0", "1", "16", "16.6666", "17", "33.34", "50", "100", "500",
];
const COUNTS: &[i32] = &[-2, -1, 0, 1, 2, 3, 7];

#[test]
fn delayer() {
    sweep("Delayer", |r| {
        let b = B::new(0, proto::DELAYER, 1, 1)
            .pin("time", r.pick(TIMES))
            .pout("time", r.pick(&["0", "40"]));
        frame(r, b, &[(0, 0, &[1, 2, 5])])
    });
}

#[test]
fn timer() {
    sweep("Timer", |r| {
        let b = B::new(0, proto::TIMER, 2, 2)
            .pin("time", r.pick(TIMES))
            .pout("time", r.pick(&["0", "40"]))
            .pout("time", "0");
        frame(r, b, &[(1, 1, &[0, 1, 1, 2]), (0, 0, &[1, 3])])
    });
}

#[test]
fn counter() {
    sweep("Counter", |r| {
        let mut b = B::new(0, proto::COUNTER, 2, 2)
            .pin("int", r.pick(COUNTS))
            .pin("int", r.pick(&[-3, 0, 1, 5]))
            .pin("int", r.pick(&[-2, -1, 0, 1, 3]));
        // without pOut 0 a loop never ends: only through a delayed link then
        let kept = r.below(4) != 0;
        if kept {
            b = b.pout("int", r.pick(&[0, 9]));
        }
        let loop_delays: &[u16] = if kept { &[0, 0, 1, 2] } else { &[1, 2] };
        frame(r, b, &[(1, 1, loop_delays), (0, 0, &[1, 4])])
    });
}

#[test]
fn sequencer() {
    sweep("Sequencer", |r| {
        let n = 1 + r.below(4);
        let mut b = B::new(0, proto::SEQUENCER, 2, n + 1);
        if r.below(4) != 0 {
            b = b.pout("int", r.pick(&[-1, 0, 2, 7]));
        }
        let mut s = frame(r, b, &[(1, 1, &[1, 2]), (0, 1, &[1, 3])]);
        if n > 1 && r.below(2) == 0 {
            let d = r.pick(&[1u16, 2]);
            s.link(Some((0, 2)), (0, 1), d);
        }
        s
    });
}

#[test]
fn binary_switch() {
    sweep("Binary Switch", |r| {
        let b = B::new(0, proto::BINARY_SWITCH, 1, 2).pin("bool", r.below(2));
        frame(r, b, &[(1, 0, &[1, 2]), (0, 0, &[3])])
    });
}

#[test]
fn wait_for_all() {
    sweep("Wait For All", |r| {
        let n = 1 + r.below(3);
        let b = B::new(0, proto::WAIT_FOR_ALL, n, 1);
        frame(r, b, &[(0, 0, &[1, 2])])
    });
}

#[test]
fn identity() {
    sweep("Identity", |r| {
        let n = r.below(3);
        let mut b = B::new(0, proto::IDENTITY, 1, 1);
        let mut types = Vec::new();
        for _ in 0..n {
            let t = r.pick(&["int", "float", "bool"]);
            types.push(t);
            // a Boolean is 0 or 1 (ours holds a bool, the C the bytes)
            let v = if t == "bool" {
                r.pick(&["0", "1"])
            } else {
                r.pick(&["-1", "0", "3"])
            };
            b = b.pin(t, v);
        }
        // sometimes one output too many or too few: the original then does nothing
        let m = match r.below(5) {
            0 => n + 1,
            1 if n > 0 => n - 1,
            _ => n,
        };
        for k in 0..m {
            let t = types.get(k).copied().unwrap_or("int");
            b = b.pout(t, if t == "bool" { "1" } else { "5" });
        }
        frame(r, b, &[(0, 0, &[1])])
    });
}

#[test]
fn group_iterator() {
    sweep("Group Iterator", |r| {
        let mut b = B::new(0, proto::GROUP_ITERATOR, 2, 2).pin("group", r.pick(&[0, 0, 1, -1]));
        // the index in pOut 1 or else local 0 (a block with neither: docs/gameplay_bbs.md, "Group Iterator")
        if r.below(2) == 0 {
            b = b.pout("obj", -1).pout("int", r.pick(&[0, 2, 5]));
        } else {
            b = b.pout("obj", -1).local("int", r.pick(&[0, 2]));
        }
        frame(r, b, &[(1, 1, &[0, 0, 1, 2]), (0, 0, &[2])])
    });
}

#[test]
fn binary_memory() {
    sweep("Binary Memory", |r| {
        let b = B::new(0, proto::BINARY_MEMORY, 2, 2).pout("bool", r.below(2));
        frame(r, b, &[(0, 1, &[1, 2]), (1, 0, &[3])])
    });
}

#[test]
fn test_block() {
    sweep("Test", |r| {
        let t = r.pick(&["int", "float"]);
        let b = B::new(0, proto::TEST, 1, 2)
            .pin("int", r.pick(&[0, 1, 2, 3, 4, 5, 6, 7]))
            .pin(t, r.pick(&["-1", "0", "2"]))
            .pin(t, r.pick(&["-1", "0", "2"]));
        frame(r, b, &[(0, 0, &[1])])
    });
}

#[test]
fn parameter_selector() {
    sweep("Parameter Selector", |r| {
        let n = 1 + r.below(3);
        let mut b = B::new(0, proto::PARAMETER_SELECTOR, n, 1);
        for k in 0..n {
            b = b.pin("int", k * 10 + 1);
        }
        b = b.pout("int", 0);
        frame(r, b, &[(0, 0, &[1])])
    });
}

#[test]
fn switch_on_parameter() {
    sweep("Switch On Parameter", |r| {
        let n = 1 + r.below(3);
        let mut b =
            B::new(0, proto::SWITCH_ON_PARAMETER, 1, n + 1).pin("int", r.pick(&[0, 1, 2, 3]));
        for _ in 0..n {
            b = b.pin("int", r.pick(&[0, 1, 2, 3]));
        }
        frame(r, b, &[(0, 0, &[1])])
    });
}

#[test]
fn per_second() {
    sweep("Per Second", |r| {
        let b = B::new(0, proto::PER_SECOND, 1, 1)
            .pin("float", r.pick(&["-3", "0", "1", "60", "1000.5"]))
            .pout("float", 0);
        frame(r, b, &[(0, 0, &[1])])
    });
}

#[test]
fn wait_message() {
    sweep("Wait Message", |r| {
        // Wait Message on S0 (object O0); Send Message from the driver script S1 at random frames, to O0 or O2
        let b = B::new(0, proto::WAIT_MESSAGE, 2, 1).pin("msg", r.pick(&["Go", "Go", "Other"]));
        let mut s = frame(r, b, &[(0, 0, &[0, 1, 2])]);
        for _ in 0..1 + r.below(3) {
            let n = s.add(B::new(1, proto::NOP, 1, 1));
            let d = r.below(20) as u16;
            s.link(None, (n, 0), d);
            let m = s.add(
                B::new(1, proto::SEND_MESSAGE, 1, 1)
                    .pin("msg", r.pick(&["Go", "Go", "Other"]))
                    .pin("obj", r.pick(&[0, 0, 2])),
            );
            s.link(Some((n, 0)), (m, 0), 0);
        }
        s
    });
}

#[test]
fn execute_script_runs_the_script_inside_the_callers_frame() {
    // Not compared with the C: it files an activation request, our notes (docs/gameplay_bbs.md, "Execute Script")
    // run the script at once. S2, off at the start, has a Nop on its start link: it runs in frame 1, after the
    // block, and the block fires Out in the first later frame in which S2 is no longer active.
    let mut s = Spec {
        objects: 3,
        scripts: vec![(0, true), (2, false)],
        frames: 3,
        ..Spec::default()
    };
    let x = s.add(
        B::new(0, proto::EXECUTE_SCRIPT, 1, 1)
            .pin("bool", 1)
            .pin("script", 1)
            .local("bool", 1),
    );
    let p = s.add(B::new(0, proto::NOP, 1, 1));
    s.link(Some((x, 0)), (p, 0), 0);
    s.link(None, (x, 0), 0);
    let n = s.add(B::new(1, proto::NOP, 1, 1));
    s.link(None, (n, 0), 0);
    assert_eq!(
        s.run(),
        "frame 1\nrun B0\nrun B2\nend S0 B0\nframe 2\nrun B0\nrun B1\nend\nframe 3\nend\n"
    );
}

#[test]
fn a_script_cannot_execute_itself_while_it_runs() {
    // docs/drahtseilakt/scripts.md, stage 0: "A script cannot run inside itself". Execute Script of its own script
    // is refused, so Out fires at once and the script is neither reset nor run again inside the frame.
    let mut s = Spec {
        objects: 1,
        scripts: vec![(0, true)],
        frames: 2,
        ..Spec::default()
    };
    let x = s.add(
        B::new(0, proto::EXECUTE_SCRIPT, 1, 1)
            .pin("bool", 1)
            .pin("script", 0)
            .local("bool", 1),
    );
    let p = s.add(B::new(0, proto::NOP, 1, 1));
    s.link(Some((x, 0)), (p, 0), 0);
    s.link(None, (x, 0), 0);
    assert_eq!(s.run(), "frame 1\nrun B0\nrun B1\nend\nframe 2\nend\n");
}

#[test]
fn a_group_iterator_without_an_index_output_or_local_walks_the_members() {
    // docs/gameplay_bbs.md, "Group Iterator": with neither pOut 1 nor local 0 our runtime keeps the index itself
    // (stage 0), so a loop still visits every member once and then fires Out.
    let mut s = Spec {
        objects: 3,
        scripts: vec![(0, true)],
        groups: vec![vec![0, 1, 2]],
        frames: 5,
        ..Spec::default()
    };
    let x = s.add(
        B::new(0, proto::GROUP_ITERATOR, 2, 2)
            .pin("group", 0)
            .pout("obj", -1),
    );
    let done = s.add(B::new(0, proto::NOP, 1, 1));
    s.link(Some((x, 0)), (done, 0), 0);
    s.link(None, (x, 0), 0);
    s.link(Some((x, 1)), (x, 1), 1);
    let t = s.run();
    let frames: Vec<&str> = t.split("frame ").skip(1).collect();
    for (f, m) in ["O0", "O1", "O2"].iter().enumerate() {
        assert!(
            frames[f].contains(&format!("pout B0 0 {m}")),
            "frame {}: {}",
            f + 1,
            frames[f]
        );
        assert!(
            !frames[f].contains("run B1"),
            "no Out yet in frame {}",
            f + 1
        );
    }
    assert!(
        frames[3].contains("run B1"),
        "Out after the last member: {}",
        frames[3]
    );
    assert!(
        frames[4].ends_with("end\npout B0 0 O2\n"),
        "nothing after: {}",
        frames[4]
    );
}

#[test]
fn op() {
    // the arithmetic on Integers and Floats (the operation table as a whole is stage 0b)
    sweep("Op", |r| {
        let ops = [
            (0x33cc_6b49u32, 0x3589_282bu32),
            (0x6764_1171, 0x6499_077a),
            (0x3899_6b85, 0x334e_35c2),
            (0x748c_3502, 0x0fea_0cf1),
        ];
        let (g0, g1) = r.pick(&ops);
        let t = r.pick(&["int", "float"]);
        let b = B::new(0, proto::OP, 1, 1)
            .pin(t, r.pick(&["-7", "0", "2", "5"]))
            .pin(t, r.pick(&["-2", "0", "1", "3"]))
            .pout(t, "0")
            .local("int", 0)
            .local("int", g0 as i32)
            .local("int", g1 as i32);
        frame(r, b, &[(0, 0, &[1])])
    });
}

/// The operation table against the C over the value types it can build: (operation, output type, input 1
/// type and values, input 2 type and values or none). Only signatures the C has a function for (directly or
/// through the parents) and two it lacks (both sides then write nothing) are compared: a row the C lacks and we
/// take from the operation's name would differ by design.
#[test]
fn op_table() {
    const I: &[&str] = &["-7", "0", "2", "5", "2147483647", "-2147483648"];
    const D: &[&str] = &["-2", "0", "1", "3", "-1"];
    // i32::MIN / -1 faults in x86 code (the C reference dies of SIGFPE): ours gives i32::MIN, not compared
    const N: &[&str] = &["-7", "0", "2", "5", "2147483647", "-2147483647"];
    const F: &[&str] = &[
        "-7.5",
        "0",
        "0.1",
        "2",
        "1e7",
        "0.00001",
        "16.666666",
        "3.14159265",
    ];
    const S: &[&str] = &["12", "-3x", "abc", "4.5", "1e3", "+7", ""];
    const V: &[&str] = &["1,2,3", "0,-0.5,7", "1e6,0,1"];
    const BX: &[&str] = &["0,1,2,3,4,5", "-1,-1,-1,1,1,1"];
    const NAMES: &[&str] = &["O0", "O2", "G0", "nope", ""];
    let (by_name, name) = (
        (0x5992_03f6u32, 0x23b0_6096u32),
        (0x5939_156eu32, 0x62ab_3472u32),
    );
    type Case = (
        (u32, u32),
        &'static str,
        (&'static str, &'static [&'static str]),
        Option<(&'static str, &'static [&'static str])>,
    );
    let (add, sub, mul, div, md, conv, len, gety, max) = (
        (0x33cc_6b49, 0x3589_282b),
        (0x6764_1171, 0x6499_077a),
        (0x3899_6b85, 0x334e_35c2),
        (0x748c_3502, 0x0fea_0cf1),
        (0x2a3f_4ab6, 0x753a_4a20),
        (0x4280_09b4, 0x1caa_5c78),
        (0x556a_69af, 0x076e_3f09),
        (0x389f_72bd, 0x7aef_3482),
        (0x6ea2_14cb, 0x43be_6b56),
    );
    let cases: Vec<Case> = vec![
        (add, "int", ("int", I), Some(("int", I))),
        (sub, "int", ("int", I), Some(("int", I))),
        (mul, "int", ("int", I), Some(("int", I))),
        (div, "int", ("int", N), Some(("int", D))),
        (md, "int", ("int", N), Some(("int", D))),
        (add, "float", ("float", F), Some(("float", F))),
        (sub, "float", ("float", F), Some(("float", F))),
        (mul, "float", ("float", F), Some(("float", F))),
        (sub, "time", ("time", F), Some(("time", F))),
        (div, "float", ("time", F), Some(("time", F))),
        (div, "float", ("float", F), Some(("float", F))),
        (mul, "float", ("int", I), Some(("float", F))),
        (mul, "float", ("float", F), Some(("int", I))),
        (mul, "int", ("float", F), Some(("int", I))),
        (add, "time", ("time", F), Some(("time", F))),
        (add, "string", ("string", S), Some(("string", S))),
        (conv, "float", ("int", I), None),
        (conv, "int", ("float", F), None),
        (conv, "float", ("bool", &["0", "1"]), None),
        (conv, "vec2", ("float", F), Some(("float", F))),
        (conv, "int", ("string", S), None),
        (conv, "string", ("int", I), None),
        (conv, "string", ("float", F), None),
        (conv, "string", ("time", F), None),
        (conv, "string", ("percentage", F), None),
        (conv, "string", ("bool", &["0", "1"]), None),
        (len, "int", ("string", S), None),
        (gety, "float", ("vector", V), None),
        (max, "vector", ("box", BX), None),
        (sub, "vector", ("vector", V), Some(("int", I))),
        // objects by name (exact names: the C compares exactly, the notes in any case) and names of objects
        (by_name, "obj", ("string", NAMES), None),
        (conv, "obj", ("string", NAMES), None),
        (name, "string", ("obj", &["0", "2", "-1"]), None),
    ];
    sweep("Op table", |r| {
        let ((g0, g1), out, (t1, v1), second) = r.pick(&cases);
        let mut b = B::new(0, proto::OP, 1, 1).pin(t1, r.pick(v1));
        if let Some((t2, v2)) = second {
            b = b.pin(t2, r.pick(v2));
        }
        let start = match out {
            "string" => "-",
            "vector" | "vec2" | "box" => "9,9,9,9,9,9",
            _ => "9",
        };
        let b = b
            .pout(out, start)
            .local("int", 0)
            .local("int", g0 as i32)
            .local("int", g1 as i32);
        frame(r, b, &[])
    });
}

/// Switch On Message on S0 (pins: message types, one output each), fed by the driver script S1 sending at
/// random frames: Send Message to O0 or O1, or Broadcast Message to a class (Behavioral Object 19, 3D Entity 33).
#[test]
fn switch_on_message() {
    sweep("Switch On Message", |r| {
        let names = ["m", "n", "m"];
        let k = 1 + r.below(3);
        let mut b = B::new(0, proto::SWITCH_ON_MESSAGE, 2, k);
        for n in names.iter().take(k) {
            b = b.pin("msg", n);
        }
        let mut s = frame(r, b, &[]);
        for _ in 0..1 + r.below(4) {
            let n = s.add(B::new(1, proto::NOP, 1, 1));
            s.link(None, (n, 0), r.below(20) as u16);
            let m = r.pick(&["m", "n", "x"]);
            let send = if r.below(2) == 0 {
                B::new(1, proto::SEND_MESSAGE, 1, 1)
                    .pin("msg", m)
                    .pin("obj", r.below(2))
            } else {
                B::new(1, proto::BROADCAST_MESSAGE, 1, 1)
                    .pin("msg", m)
                    .pin("classid", r.pick(&[19, 33]))
            };
            let x = s.add(send);
            s.link(Some((n, 0)), (x, 0), 0);
        }
        // Off now and then
        if r.below(3) == 0 {
            let x = s.blocks.len();
            let _ = x;
            s.link(None, (0, 1), 5 + r.below(20) as u16);
        }
        s
    });
}

/// The arrays of every array case: A0 3 columns x 4 rows keyed by a random column (or none), A1 2 x 0 (empty),
/// A2 1 x 1 without a key.
fn with_arrays_of(r: &mut Rng, b: B, loops: &[(usize, usize, &[u16])]) -> Spec {
    let s = frame(r, b, loops);
    with_arrays(r, s)
}

fn with_arrays(r: &mut Rng, mut s: Spec) -> Spec {
    s.arrays = vec![
        (3, 4, r.pick(&[-1, 0, 1, 2]), "", 0),
        (2, 0, 0, "", 0),
        (1, 1, -1, "", 0),
    ];
    s
}

#[test]
fn iterator() {
    sweep("Iterator", |r| {
        let b = B::new(0, proto::ITERATOR, 2, 2)
            .target("array", r.pick(&[0, 0, 1, 2]))
            .pin("int", r.pick(&[-1, 0, 1, 3, 4, 9]))
            .pin("int", r.pick(&[-1, -1, 0, 2, 3, 7]))
            .pin("bool", r.below(2))
            .pout("int", r.pick(&[0, 2]))
            .pout("int", 0)
            .pout("int", 0);
        with_arrays_of(r, b, &[(1, 1, &[0, 1, 2]), (0, 0, &[3])])
    });
}

#[test]
fn get_cell() {
    sweep("Get Cell", |r| {
        let b = B::new(0, proto::GET_CELL, 1, 2)
            .target("array", r.pick(&[0, 0, 1, 2]))
            .pin("int", r.pick(&[-1, 0, 1, 3, 4]))
            .pin("int", r.pick(&[-1, 0, 2, 3]))
            .pout("int", 5);
        with_arrays_of(r, b, &[(0, 0, &[1]), (1, 0, &[2])])
    });
}

#[test]
fn get_key_row() {
    sweep("Get Key Row", |r| {
        let b = B::new(0, proto::GET_KEY_ROW, 1, 2)
            .target("array", r.pick(&[0, 0, 1, 2]))
            .pin("int", r.pick(&[-1, 0, 10, 20, 30, 40, 12]))
            .pout("int", 5)
            .pout("int", 5)
            .pout("int", 5);
        with_arrays_of(r, b, &[(0, 0, &[1]), (1, 0, &[2])])
    });
}

// ---- arrays of every column kind (docs/gameplay_bbs.md, "Arrays") ----------------------------------------------------

/// The arrays of the typed cases: A0 4 columns (Integer, Float, String, Object) x 12 rows with repeated values, keyed
/// by a random column (or none), A1 an empty one, A2 1 x 1, A3 3 columns x 30 rows (enough to sort by partitions).
fn with_mixed_arrays(r: &mut Rng, mut s: Spec) -> Spec {
    s.arrays = vec![
        (4, 12, r.pick(&[-1, 0, 1, 2, 3]), "ifso", 5),
        (2, 0, 0, "fs", 0),
        (1, 1, -1, "", 0),
        (3, 30, r.pick(&[-1, 0, 1]), "sif", 6),
    ];
    s
}

fn typed_loops(r: &mut Rng, b: B, loops: &[(usize, usize, &[u16])]) -> Spec {
    let s = frame(r, b, loops);
    with_mixed_arrays(r, s)
}

fn array_target(r: &mut Rng) -> i32 {
    r.pick(&[0, 0, 0, 3, 3, 1, 2, -1])
}

/// A pin of a random type with a value the sample arrays hold (or just misses).
fn typed_pin(r: &mut Rng, b: B) -> B {
    match r.below(4) {
        0 => b.pin("int", r.pick(&[0, 1, 2, 3, -1])),
        1 => b.pin(
            "float",
            r.pick(&["0", "0.5", "1", "1.5", "2", "3.5", "2.5"]),
        ),
        2 => b.pin(
            "string",
            r.pick(&["s0", "s1", "s2", "s3", "s4", "s5", "", "x"]),
        ),
        _ => b.pin("obj", r.pick(&[0, 1, 2, -1])),
    }
}

fn typed_pout(r: &mut Rng, b: B) -> B {
    match r.below(4) {
        0 => b.pout("int", 9),
        1 => b.pout("float", 9),
        2 => b.pout("string", "z"),
        _ => b.pout("obj", -1),
    }
}

#[test]
fn set_row() {
    sweep("Set Row", |r| {
        let mut b = B::new(0, proto::SET_ROW, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 3, 11, 12, 20]));
        for _ in 0..4 {
            b = typed_pin(r, b);
        }
        b = b.local("bool", r.below(2));
        typed_loops(r, b, &[(0, 0, &[1, 2]), (1, 0, &[3])])
    });
}

#[test]
fn insert_column() {
    sweep("Insert Column", |r| {
        let b = B::new(0, proto::INSERT_COLUMN, 1, 1)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 4, 5, 9]))
            .pin("string", "n")
            .pin("int", r.pick(&[1, 2, 3, 4]))
            .pin("int", 0);
        typed_loops(r, b, &[(0, 0, &[1, 2])])
    });
}

#[test]
fn remove_column() {
    sweep("Remove Column", |r| {
        let b = B::new(0, proto::REMOVE_COLUMN, 1, 1)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4, 9]));
        typed_loops(r, b, &[(0, 0, &[1, 2])])
    });
}

#[test]
fn sort_rows() {
    sweep("Sort Rows", |r| {
        let b = B::new(0, proto::SORT_ROWS, 1, 1)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pin("bool", r.below(2));
        typed_loops(r, b, &[(0, 0, &[1, 2])])
    });
}

#[test]
fn test_cell() {
    sweep("Test Cell", |r| {
        let b = B::new(0, proto::TEST_CELL, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 5, 11, 12]))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pin("int", r.pick(&[0, 1, 2, 3, 4, 5, 6, 7]));
        let b = typed_pin(r, b);
        typed_loops(r, b, &[(0, 0, &[1]), (1, 0, &[2])])
    });
}

#[test]
fn get_highest() {
    sweep("Get Highest", |r| {
        let mut b = B::new(0, proto::GET_HIGHEST, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pout("int", 7);
        for _ in 0..4 {
            b = typed_pout(r, b);
        }
        typed_loops(r, b, &[(0, 0, &[1, 2]), (1, 0, &[3])])
    });
}

#[test]
fn value_count() {
    sweep("Value Count", |r| {
        let b = B::new(0, proto::VALUE_COUNT, 1, 1)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pin("int", r.pick(&[0, 1, 2, 3, 4, 5, 6, 7]));
        let b = typed_pin(r, b).pout("int", 9);
        typed_loops(r, b, &[(0, 0, &[1, 2])])
    });
}

#[test]
fn remove_key_row() {
    sweep("Remove Key Row", |r| {
        let b = B::new(0, proto::REMOVE_KEY_ROW, 1, 2).target("array", array_target(r));
        let b = typed_pin(r, b);
        typed_loops(r, b, &[(0, 0, &[1, 2]), (1, 0, &[3])])
    });
}

#[test]
fn remove_row_if() {
    sweep("Remove Row If", |r| {
        let b = B::new(0, proto::REMOVE_ROW_IF, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pin("int", r.pick(&[0, 1, 2, 3, 4, 5, 6, 7]));
        let b = typed_pin(r, b).pout("int", 9);
        typed_loops(r, b, &[(0, 0, &[1, 2]), (1, 0, &[3])])
    });
}

#[test]
fn change_value_if() {
    sweep("Change Value If", |r| {
        let b = B::new(0, proto::CHANGE_VALUE_IF, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pin("int", r.pick(&[0, 1, 2, 3, 4, 5, 6, 7]));
        let b = typed_pin(r, b);
        let b = typed_pin(r, b).pout("int", 9);
        typed_loops(r, b, &[(0, 0, &[1, 2]), (1, 0, &[3])])
    });
}

#[test]
fn row_search() {
    sweep("Row Search", |r| {
        let b = B::new(0, proto::ROW_SEARCH, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pin("int", r.pick(&[-1, 0, 1, 5, 12]))
            .pin("int", r.pick(&[0, 1, 2, 3, 4, 5, 6, 7]));
        let b = typed_pin(r, b).pout("int", 9);
        typed_loops(r, b, &[(0, 0, &[1, 2]), (1, 0, &[3])])
    });
}

#[test]
fn typed_get_key_row_and_get_cell() {
    sweep("Get Key Row (typed)", |r| {
        let b = B::new(0, proto::GET_KEY_ROW, 1, 2).target("array", array_target(r));
        let mut b = typed_pin(r, b);
        for _ in 0..4 {
            b = typed_pout(r, b);
        }
        typed_loops(r, b, &[(0, 0, &[1]), (1, 0, &[2])])
    });
    sweep("Get Cell (typed)", |r| {
        let b = B::new(0, proto::GET_CELL, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 11, 12]))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]));
        let b = typed_pout(r, b);
        typed_loops(r, b, &[(0, 0, &[1]), (1, 0, &[2])])
    });
    sweep("Set Cell (typed)", |r| {
        let b = B::new(0, proto::SET_CELL, 1, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 11, 12]))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]));
        let b = typed_pin(r, b);
        typed_loops(r, b, &[(0, 0, &[1]), (1, 0, &[2])])
    });
    sweep("Iterator (typed)", |r| {
        let mut b = B::new(0, proto::ITERATOR, 2, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[0, 1, 3]))
            .pin("int", r.pick(&[-1, 2, 5]))
            .pin("bool", r.below(2))
            .pout("int", 0);
        for _ in 0..4 {
            b = typed_pout(r, b);
        }
        typed_loops(r, b, &[(1, 1, &[0, 1])])
    });
}

/// Move Row has no C reference (the upstream runtime lacks it): the expectation is from the docs alone (the row at
/// Src Row is taken out and put at Dest Row; an index outside the array changes nothing; Out fires either way).
#[test]
fn move_row_follows_the_notes() {
    let run = |src: i32, dest: i32| {
        let b = B::new(0, proto::MOVE_ROW, 1, 1)
            .target("array", 0)
            .pin("int", src)
            .pin("int", dest);
        let mut s = frame(&mut Rng::new(1), b, &[]);
        s.arrays = vec![(1, 4, -1, "", 0)];
        s.links.clear();
        s.link(None, (0, 0), 0);
        s.frames = 3;
        let t = s.run();
        let last = t.rsplit("frame 3").next().unwrap().to_string();
        let rows: Vec<&str> = last.lines().filter(|l| l.starts_with("row")).collect();
        (
            rows.join("|"),
            last.contains("run B0") || t.contains("run B0"),
        )
    };
    assert_eq!(run(3, 0).0, "row 30|row 0|row 10|row 20");
    assert_eq!(run(0, 2).0, "row 10|row 20|row 0|row 30");
    assert_eq!(run(1, 1).0, "row 0|row 10|row 20|row 30");
    assert_eq!(run(4, 0).0, "row 0|row 10|row 20|row 30");
    assert_eq!(run(-1, 0).0, "row 0|row 10|row 20|row 30");
    assert_eq!(run(0, 9).0, "row 0|row 10|row 20|row 30");
}

/// Our side alone on a block fired once at the start: the pout lines of the last frame.
fn once(b: B) -> String {
    let mut s = frame(&mut Rng::new(1), b, &[]);
    s.links.clear();
    s.link(None, (0, 0), 0);
    s.frames = 3;
    let t = s.run();
    let last = t.rsplit("frame 3").next().unwrap().to_string();
    last.lines()
        .filter(|l| l.starts_with("pout"))
        .collect::<Vec<_>>()
        .join("|")
}

const INT_FORMATS: &[&str] = &[
    "%d", "", "%5d", "%-5d|", "%05d", "%+d", "%x", "%X", "%o", "%3.2d", "%#x", "%u", "n=%d!", "%%%d",
];
const FLOAT_FORMATS: &[&str] = &[
    "%f", "%g", "%.2f", "%8.3f", "%e", "%G", "%10.4e", "", "%5.1f", "%+g", "%-9.2f|", "%.0f", "%#g", "%08.3f",
    "%.10g", "%E",
];
const STRING_FORMATS: &[&str] = &["%s", "", "%5s", "%-5s", "[%s]", ">%.3s"];

#[test]
fn create_string() {
    sweep("Create String", |r| {
        let mut b = B::new(0, proto::CREATE_STRING, 3, 3)
            .pin("string", r.pick(&["", ",", "|", ";;", "-"]));
        let mut formats = Vec::new();
        for _ in 0..1 + r.below(4) {
            match r.below(4) {
                0 => {
                    b = b.pin("int", r.pick(&["-7", "0", "42", "255", "-1", "65"]));
                    formats.push(r.pick(INT_FORMATS));
                }
                1 => {
                    b = b.pin(
                        "float",
                        r.pick(&["-7.5", "0", "0.1", "3.14159", "1e7", "123456.789", "0.00001234", "1e-7", "100"]),
                    );
                    formats.push(r.pick(FLOAT_FORMATS));
                }
                2 => {
                    b = b.pin("string", r.pick(&["", "abc", "hello_world", "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]));
                    formats.push(r.pick(STRING_FORMATS));
                }
                _ => {
                    b = b.pin("bool", r.below(2));
                    formats.push(r.pick(&["", "%s", "x%s"]));
                }
            }
        }
        b = b.pout("string", r.pick(&["-", "", "old"]));
        for f in formats {
            b = b.local("string", f);
        }
        frame(r, b, &[(1, 1, &[1, 2]), (0, 2, &[2])])
    });
}

#[test]
fn create_string_formats_and_joins() {
    fn run(delim: &str, parts: &[(&'static str, &str, &str)], inputs: usize) -> String {
        let mut b = B::new(0, proto::CREATE_STRING, 3, 3).pin("string", delim);
        for (t, v, _) in parts {
            b = b.pin(t, v);
        }
        b = b.pout("string", if inputs == 1 { "" } else { "-" });
        for (_, _, f) in parts {
            b = b.local("string", f);
        }
        let mut s = frame(&mut Rng::new(1), b, &[]);
        s.links.clear();
        s.link(None, (0, inputs), 0);
        s.frames = 3;
        let t = s.run();
        let last = t.rsplit("frame 3").next().unwrap().to_string();
        last.lines().find(|l| l.starts_with("pout")).unwrap().to_string()
    }
    // Landfill's message: strings with "%s", the delimiter between every pair
    assert_eq!(
        run(", ", &[("string", "a", "%s"), ("int", "7", "%03d"), ("float", "2.5", "%.1f"), ("string", "z", "")], 0),
        "pout B0 0 a, 007, 2.5, z"
    );
    assert_eq!(run("", &[("float", "0.5", ""), ("int", "-3", "")], 0), "pout B0 0 0.5-3");
    // an Integer or Float with an empty format is its own text; a Boolean is TRUE / FALSE
    assert_eq!(run("/", &[("bool", "1", ""), ("bool", "0", "%s")], 0), "pout B0 0 TRUE/FALSE");
    // Add Line (input 1) to an empty text has no newline; Clear (input 2) empties it
    assert_eq!(run("", &[("string", "line", "%s")], 1), "pout B0 0 line");
    assert_eq!(run("", &[("string", "line", "%s")], 2), "pout B0 0 ");
    // a hostile format: only the first conversion takes the value, a width is capped, and nothing panics
    assert_eq!(run("", &[("int", "5", "%d%d|%")], 0), "pout B0 0 5|");
    let long = run("", &[("int", "5", "%99999999d")], 0);
    assert!(long.len() < 1100, "{}", long.len());
    assert_eq!(run("", &[("int", "5", "%.*d %n%ll")], 0), "pout B0 0 5 ");
    // huge parts and delimiters: the text stops at 64 KiB
    let big = "y".repeat(60_000);
    let big = big.as_str();
    let t = run(big, &[("string", big, "%s"), ("string", big, ""), ("string", big, "")], 0);
    assert_eq!(t.len(), "pout B0 0 ".len() + (1 << 16));
    // a multi-byte character in a format or a text is kept whole
    assert_eq!(run("", &[("string", "\u{e9}t\u{e9}", "\u{e9}>%s")], 0), "pout B0 0 \u{e9}>\u{e9}t\u{e9}");
}

#[test]
fn scan_string() {
    sweep("Scan String", |r| {
        let mut b = B::new(0, proto::SCAN_STRING, 1, 1)
            .pin(
                "string",
                r.pick(&["a,b,,c", "1,2,,3", "x_y_z", "", "_lead", "12_4.5_abc", "ab--cd--ef", "trail-", "a", ",,,", "1e2_-3_+4_x7"]),
            )
            .pin("string", r.pick(&[",", "_", "", "--", "ab", "zz", "-"]))
            .pout("int", 7);
        for _ in 0..r.below(5) {
            b = match r.below(3) {
                0 => b.pout("int", 9),
                1 => b.pout("float", 9),
                _ => b.pout("string", "z"),
            };
        }
        frame(r, b, &[(0, 0, &[1])])
    });
}

#[test]
fn scan_string_splits_and_converts() {
    let b = B::new(0, proto::SCAN_STRING, 1, 1)
        .pin("string", "12 4.5  abc")
        .pin("string", "")
        .pout("int", 7)
        .pout("int", 9)
        .pout("float", 9)
        .pout("string", "z");
    assert_eq!(once(b), "pout B0 0 3|pout B0 1 12|pout B0 2 40900000|pout B0 3 abc");
    // more pieces than outputs: the count is not written
    let b = B::new(0, proto::SCAN_STRING, 1, 1)
        .pin("string", "a,b,c")
        .pin("string", ",")
        .pout("int", 7)
        .pout("string", "z");
    assert_eq!(once(b), "pout B0 0 7|pout B0 1 a");
    // a multi-byte delimiter, leading and trailing delimiters
    let b = B::new(0, proto::SCAN_STRING, 1, 1)
        .pin("string", "--a----b--")
        .pin("string", "--")
        .pout("int", 7)
        .pout("string", "z")
        .pout("string", "z");
    assert_eq!(once(b), "pout B0 0 2|pout B0 1 a|pout B0 2 b");
}

#[test]
fn get_substring() {
    sweep("Get SubString", |r| {
        let b = B::new(0, proto::GET_SUBSTRING, 1, 1)
            .pin("string", r.pick(&["Hello_World", "", "a", "Ball_Wood_Sub1", "0123456789"]))
            .pin("int", r.pick(&[-50, -5, -1, 0, 1, 3, 10, 11, 12, 100]))
            .pin("int", r.pick(&[-30, -3, -1, 0, 1, 2, 5, 8, 11, 50, 1000]))
            .pout("string", "-");
        frame(r, b, &[(0, 0, &[1])])
    });
}

#[test]
fn get_substring_hand_values_and_hostile_input() {
    let sub = |s: &str, start: i32, len: i32| {
        once(
            B::new(0, proto::GET_SUBSTRING, 1, 1)
                .pin("string", s)
                .pin("int", start)
                .pin("int", len)
                .pout("string", "-"),
        )
    };
    assert_eq!(sub("Hello World", 6, 0), "pout B0 0 World");
    assert_eq!(sub("Hello World", 0, 5), "pout B0 0 Hello");
    assert_eq!(sub("Hello World", 6, 50), "pout B0 0 World");
    assert_eq!(sub("Hello World", -2, 5), "pout B0 0 Hel");
    assert_eq!(sub("Hello World", 11, 3), "pout B0 0 ");
    assert_eq!(sub("Hello World", 99, 0), "pout B0 0 ");
    assert_eq!(sub("", 0, 0), "pout B0 0 ");
    // extreme numbers wrap as the original's 32-bit arithmetic does and never panic
    for (a, b) in [(i32::MAX, i32::MAX), (i32::MIN, i32::MIN), (i32::MAX, 0), (i32::MIN, 5), (3, i32::MAX), (-1, i32::MIN)] {
        let _ = sub("Hello World", a, b);
    }
    // not UTF-8 text reaches the runtime already made lossy: a slice never cuts a character in the middle
    let part = script::blocks::strings::substring(&[0xe9, b'a', 0xff, b'b', 0xc3], 1, 3);
    assert_eq!(part, vec![b'a', 0xff, b'b']);
}

/// A random expression over the C reference's grammar (numbers, a, b, PI, + - * /, parentheses, unary signs), with
/// spaces; divisors are plain nonzero numbers or variables, so no NaN is made.
fn expression(r: &mut Rng, depth: usize) -> String {
    let atom = |r: &mut Rng| -> String {
        r.pick(&["0", "1", "2", "2.5", "0.1", "10", ".5", "a", "b", "A", "pi", "PI", "3"]).to_string()
    };
    let nonzero = |r: &mut Rng| -> String { r.pick(&["1", "2", "2.5", "0.1", "10", "a", "b", "pi"]).to_string() };
    if depth == 0 || r.below(4) == 0 {
        return atom(r);
    }
    // (the harness reads a value up to the next blank: no spaces here; the spaces are in the independent test)
    match r.below(6) {
        0 => format!("({})", expression(r, depth - 1)),
        1 => format!("-{}", atom(r)),
        2 => format!("{}*{}", expression(r, depth - 1), atom(r)),
        3 => format!("{}/{}", expression(r, depth - 1), nonzero(r)),
        4 => format!("{}+{}", expression(r, depth - 1), expression(r, depth - 1)),
        _ => format!("{}-{}", expression(r, depth - 1), expression(r, depth - 1)),
    }
}

#[test]
fn calculator() {
    sweep("Calculator", |r| {
        // sometimes a broken expression: both sides log it and leave x alone
        let e = match r.below(8) {
            0 => r.pick(&["", "1+", "(1", "1)", "c", "ab", "*2", "1..2", "2a", "a*"]).to_string(),
            _ => expression(r, 3),
        };
        let t = r.pick(&["float", "int"]);
        let b = B::new(0, proto::CALCULATOR, 1, 1)
            .pin("string", e)
            .pin(r.pick(&["float", "int"]), r.pick(&["-3", "1", "2", "7"]))
            .pin(t, r.pick(&["1", "2", "5"]))
            .pout("float", "9")
            .local("string", "")
            .local("int", 0);
        frame(r, b, &[(0, 0, &[1])])
    });
}

/// An expression as a tree, evaluated by a second implementation (the oracle of this test): every binary result is
/// rounded to single precision, as the block does.
#[derive(Clone, Debug)]
enum Ex {
    Num(f32),
    Var(usize),
    Neg(Box<Ex>),
    Bin(char, Box<Ex>, Box<Ex>),
    Fun(&'static str, Box<Ex>),
}

fn gen_ex(r: &mut Rng, depth: usize) -> Ex {
    if depth == 0 || r.below(5) == 0 {
        return if r.below(2) == 0 {
            Ex::Num(r.pick(&[0.5f32, 1.0, 2.0, 3.0, 0.25, 10.0, 1.5]))
        } else {
            Ex::Var(r.below(3))
        };
    }
    match r.below(7) {
        0 => Ex::Neg(Box::new(gen_ex(r, depth - 1))),
        1 => Ex::Fun(
            r.pick(&["sin", "cos", "abs", "sqrt", "ln", "exp", "atan", "floor"]),
            Box::new(gen_ex(r, depth - 1)),
        ),
        _ => Ex::Bin(
            r.pick(&['+', '-', '*', '/', '^']),
            Box::new(gen_ex(r, depth - 1)),
            Box::new(gen_ex(r, depth - 1)),
        ),
    }
}

fn eval_ex(e: &Ex, v: &[f32; 3]) -> f64 {
    let rd = |x: f64| f64::from(x as f32);
    match e {
        Ex::Num(n) => f64::from(*n),
        Ex::Var(i) => f64::from(v[*i]),
        Ex::Neg(a) => -eval_ex(a, v),
        Ex::Fun(f, a) => {
            let a = eval_ex(a, v);
            rd(match *f {
                "sin" => a.sin(),
                "cos" => a.cos(),
                "abs" => a.abs(),
                "sqrt" => a.sqrt(),
                "ln" => a.ln(),
                "exp" => a.exp(),
                "atan" => a.atan(),
                _ => a.floor(),
            })
        }
        Ex::Bin(o, a, b) => {
            let (a, b) = (eval_ex(a, v), eval_ex(b, v));
            rd(match o {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                _ => a.powf(b),
            })
        }
    }
}

/// Printed with parentheses everywhere a precedence could matter, with random extra ones and spaces.
fn print_ex(e: &Ex, r: &mut Rng) -> String {
    let sp = |r: &mut Rng| r.pick(&["", " "]);
    match e {
        Ex::Num(n) => format!("{n}"),
        Ex::Var(i) => ["a", "b", "c"][*i].to_uppercase_if(r.below(2) == 0),
        Ex::Neg(a) => format!("(-{})", print_ex(a, r)),
        Ex::Fun(f, a) => format!("{}{}({})", f.to_uppercase_if(r.below(2) == 0), sp(r), print_ex(a, r)),
        Ex::Bin(o, a, b) => format!("({}{}{o}{}{})", print_ex(a, r), sp(r), sp(r), print_ex(b, r)),
    }
}

trait CaseIf {
    fn to_uppercase_if(&self, up: bool) -> String;
}
impl CaseIf for str {
    fn to_uppercase_if(&self, up: bool) -> String {
        if up { self.to_uppercase() } else { self.to_string() }
    }
}

/// The calculator against an independent evaluation of random expression trees over the whole grammar (the C
/// reference knows only part of it): the result is the same single-precision number, or both are not numbers.
#[test]
fn calculator_matches_an_independent_evaluation_on_random_expressions() {
    for seed in 0..400u64 {
        let mut r = Rng::new(seed);
        let tree = gen_ex(&mut r, 4);
        let vals = [r.pick(&[0.5f32, 2.0, 3.0, 0.75]), r.pick(&[1.0f32, 4.0, 2.5]), r.pick(&[2.0f32, 0.125])];
        let text = print_ex(&tree, &mut r);
        let want = eval_ex(&tree, &vals) as f32;
        let b = B::new(0, proto::CALCULATOR, 1, 1)
            .pin("string", &text)
            .pin("float", vals[0])
            .pin("float", vals[1])
            .pin("float", vals[2])
            .pout("float", "9")
            .local("string", "");
        let got = once(b);
        let shown = format!("pout B0 0 {:08x}", want.to_bits());
        if want.is_nan() {
            // NaN is whatever the hardware makes: only "a NaN" is checked
            let bits = u32::from_str_radix(got.rsplit(' ').next().unwrap(), 16).unwrap();
            assert!(f32::from_bits(bits).is_nan(), "{text}: {got}");
        } else {
            assert_eq!(got, shown, "seed {seed}: {text}");
        }
    }
}

#[test]
fn calculator_hand_values_and_caps() {
    let calc = |e: &str, a: f32, b: f32| {
        once(
            B::new(0, proto::CALCULATOR, 1, 1)
                .pin("string", e)
                .pin("float", a)
                .pin("float", b)
                .pout("float", "9")
                .local("string", ""),
        )
    };
    let bits = |x: f32| format!("pout B0 0 {:08x}", x.to_bits());
    // the maps' expressions
    assert_eq!(calc("(a^2+a)*2", 3.0, 0.0), bits(24.0));
    assert_eq!(calc("sin(a*b)/2+0.5", 0.0, 5.0), bits(0.5));
    let n = (-2.0f64 * 0.25f64.ln()).sqrt() * (2.0 * f64::from(3.141_592_7_f32) * 0.0).cos();
    assert_eq!(calc("sqrt(-2*ln(a))*cos(2*pi*b)", 0.25, 0.0), bits(n as f32));
    // a syntax error leaves x as it was (9, the pout's start in the harness: the bits of 9.0 are not written)
    assert_eq!(calc("1+", 0.0, 0.0), format!("pout B0 0 {:08x}", 9.0f32.to_bits()));
    // caps: 20 nested parentheses are fine, 40 are not; the expression is read to 511 bytes
    let deep = |n: usize| format!("{}a{}", "(".repeat(n), ")".repeat(n));
    assert_eq!(calc(&deep(20), 4.0, 0.0), bits(4.0));
    assert_eq!(calc(&deep(40), 4.0, 0.0), format!("pout B0 0 {:08x}", 9.0f32.to_bits()));
    let long = vec!["1"; 129].join("+");
    assert_eq!(calc(&long, 0.0, 0.0), format!("pout B0 0 {:08x}", 9.0f32.to_bits()));
    assert_eq!(calc(&"9".repeat(100_000), 0.0, 0.0), calc(&"9".repeat(511), 0.0, 0.0));
}

#[test]
fn iterator_if_sweep() {
    sweep("Iterator If", |r| {
        let b = B::new(0, proto::ITERATOR_IF, 2, 2)
            .target("array", array_target(r))
            .pin("int", r.pick(&[-1, 0, 1, 2, 3, 4]))
            .pin("int", r.pick(&[0, 1, 2, 3, 4, 5, 6, 7]));
        let b = typed_pin(r, b).pout("int", 9);
        let mut b = b;
        for _ in 0..3 {
            b = typed_pout(r, b);
        }
        typed_loops(r, b, &[(1, 1, &[0, 1, 2]), (0, 0, &[3])])
    });
}

/// Shuffle Order has no C reference: it keeps every row, in some order, the same one for the same run.
#[test]
fn shuffle_order_keeps_the_rows() {
    let build = || {
        let b = B::new(0, proto::SHUFFLE_ORDER, 1, 1).target("array", 0);
        let mut s = frame(&mut Rng::new(1), b, &[]);
        s.arrays = vec![(1, 8, -1, "", 0)];
        s.links.clear();
        s.link(None, (0, 0), 0);
        s.frames = 3;
        s.run()
    };
    let t = build();
    let rows = |t: &str| -> Vec<String> {
        t.rsplit("frame 3").next().unwrap().lines().filter(|l| l.starts_with("row")).map(String::from).collect()
    };
    let mut got = rows(&t);
    assert_eq!(t, build(), "the same run shuffles the same way");
    assert_ne!(got, (0..8).map(|i| format!("row {}", 10 * i)).collect::<Vec<_>>(), "it moved rows");
    got.sort();
    let mut want: Vec<String> = (0..8).map(|i| format!("row {}", 10 * i)).collect();
    want.sort();
    assert_eq!(got, want);
}
