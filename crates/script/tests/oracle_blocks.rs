// The differential oracle, level 2 (docs/drahtseilakt/scripts.md, "C oracle, level 2"): one synthetic graph per block
// kind, its pins swept over edge values (zero and negative times and counts, Reset during a run, re-entry in the same
// frame, the script deactivated mid-run), run in our runtime and in the upstream C runtime (tests/oracle/ck_oracle.c,
// a test-only reference built with cmake). The traces must be equal frame by frame: the blocks run, in order, the
// scripts and blocks active after the frame and every output parameter of the block under test. The C code is itself
// a reimplementation: a disagreement is settled by the notes (docs/), and only where they are silent by the C, with
// the finding written down. Skipped unless OPENBALLANCE_ORACLE names the built ck_oracle.

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
    Block, Graph, Guid, Id, Io, Link, Node, ObjInfo, ObjKind, Param, Runtime, Value, types,
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
            objects.push(ObjInfo {
                name,
                class: 0,
                kind,
            });
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
            nodes.push((
                id,
                Node::Block(Block {
                    name: format!("B{i}"),
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
        let mut g = Graph::assemble(objects, nodes, Vec::new(), scripts).expect("assembles");
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
                    "float" | "time" => format!("{:08x}", v.as_float().to_bits()),
                    "string" => v.as_string(),
                    "obj" | "script" | "group" => v
                        .as_obj()
                        .map_or("-".to_string(), |o| rt.graph.name(o).to_string()),
                    _ => v.as_int().to_string(),
                };
                writeln!(out, "pout B{b} {k} {shown}").unwrap();
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
    fn sound(&mut self, _: &str, _: bool) {}
    fn text(&mut self, _: &str, _: &str, _: bool) {}
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
        // without pOut 0 a loop never ends: only through a delayed link then (the iteration cap is not compared:
        // docs/ck-runtime.md says at most BehaviorMaxIteration pops, the C makes one more)
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
