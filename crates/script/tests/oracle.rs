// The differential oracle, level 1 (docs/drahtseilakt/scripts.md, "Stage 0"): random graphs of trivial blocks (Nop,
// Binary Switch, Activate Script, Deactivate Script) with random link delays, block priorities and start states
// run in our runtime and in the upstream C runtime (tests/oracle/ck_oracle.c, a test-only reference built with
// cmake), and their traces must be equal frame by frame: the blocks run in each frame, in order, and the scripts
// and blocks active after it. The C code is itself a reimplementation; a disagreement is settled by the notes
// (docs/ck-runtime.md), not by the oracle. Skipped unless OPENBALLANCE_ORACLE names the built ck_oracle.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::type_complexity
)]

use script::blocks::proto;
use script::{
    Block, Graph, Guid, Id, Io, Link, Node, ObjInfo, ObjKind, Param, Runtime, Value, types,
};
use std::fmt::Write as _;
use std::sync::Arc;

/// A graph description, the text both sides read (the format: tests/oracle/ck_oracle.c).
#[derive(Default)]
struct Spec {
    objects: usize,
    /// (object, active at the start)
    scripts: Vec<(usize, bool)>,
    /// (script, kind, priority, a, b)
    blocks: Vec<(usize, &'static str, i32, usize, usize)>,
    /// (script, from: None = the start, Some((block, output)), to block, delay)
    links: Vec<(usize, Option<(usize, usize)>, usize, u16)>,
    frames: u32,
}

impl Spec {
    fn text(&self) -> String {
        let mut t = String::new();
        for o in 0..self.objects {
            writeln!(t, "object O{o}").unwrap();
        }
        for (i, (o, on)) in self.scripts.iter().enumerate() {
            writeln!(t, "script S{i} {o} {}", u8::from(*on)).unwrap();
        }
        for (i, (s, k, p, a, b)) in self.blocks.iter().enumerate() {
            writeln!(t, "block B{i} {s} {k} {p} {a} {b}").unwrap();
        }
        for (s, from, to, d) in &self.links {
            let from = from.map_or("s".to_string(), |(b, o)| format!("{b}.{o}"));
            writeln!(t, "link {s} {from} {to} {d}").unwrap();
        }
        writeln!(t, "frames {}", self.frames).unwrap();
        t
    }

    /// The same graph for our runtime.
    fn graph(&self) -> (Graph, Vec<Id>, Vec<Id>) {
        let mut objects: Vec<ObjInfo> = Vec::new();
        let mut nodes: Vec<(Id, Node)> = Vec::new();
        fn add(
            objects: &mut Vec<ObjInfo>,
            nodes: &mut Vec<(Id, Node)>,
            name: String,
            node: Option<Node>,
        ) -> Id {
            let id = objects.len();
            objects.push(ObjInfo {
                name,
                class: 0,
                kind: ObjKind::Other,
            });
            if let Some(n) = node {
                nodes.push((id, n));
            }
            id
        }
        for o in 0..self.objects {
            add(&mut objects, &mut nodes, format!("O{o}"), None);
        }
        // ids are handed out up front so a script can be named before its blocks exist
        let script_ids: Vec<Id> = (0..self.scripts.len())
            .map(|i| add(&mut objects, &mut nodes, format!("S{i}"), None))
            .collect();
        let script_in: Vec<Id> = (0..self.scripts.len())
            .map(|_| {
                add(
                    &mut objects,
                    &mut nodes,
                    "In".into(),
                    Some(Node::Io(Io::default())),
                )
            })
            .collect();
        let mut subs = vec![Vec::new(); self.scripts.len()];
        let mut links = vec![Vec::new(); self.scripts.len()];
        let mut block_ids = Vec::new();
        let mut ios = Vec::new();
        for (i, &(s, kind, prio, a, b)) in self.blocks.iter().enumerate() {
            let id = add(&mut objects, &mut nodes, format!("B{i}"), None);
            let inp = add(
                &mut objects,
                &mut nodes,
                "In".into(),
                Some(Node::Io(Io::default())),
            );
            let outs: Vec<Id> = (0..if kind == "switch" { 2 } else { 1 })
                .map(|_| {
                    add(
                        &mut objects,
                        &mut nodes,
                        "Out".into(),
                        Some(Node::Io(Io::default())),
                    )
                })
                .collect();
            let mut pins = Vec::new();
            let pin = |g: Guid,
                       v: Value,
                       objects: &mut Vec<ObjInfo>,
                       nodes: &mut Vec<(Id, Node)>,
                       pins: &mut Vec<Id>| {
                let local = add(
                    objects,
                    nodes,
                    "value".into(),
                    Some(Node::Param(Param {
                        name: "value".into(),
                        guid: g,
                        value: v,
                        ..Param::default()
                    })),
                );
                pins.push(add(
                    objects,
                    nodes,
                    "pin".into(),
                    Some(Node::Param(Param {
                        name: "pin".into(),
                        guid: g,
                        source: Some(local),
                        ..Param::default()
                    })),
                ));
            };
            let guid = match kind {
                "nop" => proto::NOP,
                "switch" => {
                    pin(
                        types::BOOL,
                        Value::Bool(a != 0),
                        &mut objects,
                        &mut nodes,
                        &mut pins,
                    );
                    proto::BINARY_SWITCH
                }
                "activate" => {
                    pin(
                        types::BOOL,
                        Value::Bool(b != 0),
                        &mut objects,
                        &mut nodes,
                        &mut pins,
                    );
                    pin(
                        types::SCRIPT,
                        Value::Obj(Some(script_ids[a])),
                        &mut objects,
                        &mut nodes,
                        &mut pins,
                    );
                    proto::ACTIVATE_SCRIPT
                }
                "deactivate" => {
                    pin(
                        types::SCRIPT,
                        Value::Obj(Some(script_ids[a])),
                        &mut objects,
                        &mut nodes,
                        &mut pins,
                    );
                    proto::DEACTIVATE_SCRIPT
                }
                _ => unreachable!(),
            };
            nodes.push((
                id,
                Node::Block(Block {
                    name: format!("B{i}"),
                    proto: Some(guid),
                    priority: prio,
                    inputs: vec![inp],
                    outputs: outs.clone(),
                    pins,
                    ..Block::default()
                }),
            ));
            subs[s].push(id);
            block_ids.push(id);
            ios.push((inp, outs));
        }
        for &(s, from, to, delay) in &self.links {
            let src = match from {
                None => script_in[s],
                Some((b, o)) => ios[b].1[o],
            };
            let l = add(
                &mut objects,
                &mut nodes,
                "link".into(),
                Some(Node::Link(Link {
                    delay,
                    src,
                    dst: ios[to].0,
                })),
            );
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
            .map(|(&id, &(o, _))| (id, Some(o)))
            .collect();
        let mut g = Graph::assemble(objects, nodes, Vec::new(), scripts).expect("assembles");
        g.start_off = script_ids
            .iter()
            .zip(&self.scripts)
            .filter(|(_, (_, on))| !on)
            .map(|(&id, _)| id)
            .collect();
        (g, script_ids, block_ids)
    }

    /// Our trace in the oracle's form.
    fn run(&self) -> String {
        let (g, scripts, blocks) = self.graph();
        let mut rt = Runtime::new(Arc::new(g));
        rt.set_trace(true);
        let mut host = script::Quiet;
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
        }
        out
    }
}

/// A small generator (xorshift), so a failing case is found again by its seed.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x % n.max(1) as u64) as usize
    }
}

fn random_spec(seed: u64) -> Spec {
    let mut r = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut s = Spec {
        objects: 1 + r.below(3),
        frames: 40,
        ..Spec::default()
    };
    let nscripts = 1 + r.below(4);
    for _ in 0..nscripts {
        s.scripts.push((r.below(s.objects), r.below(4) != 0));
    }
    for sc in 0..nscripts {
        let first = s.blocks.len();
        let n = 1 + r.below(6);
        for _ in 0..n {
            let kind = ["nop", "nop", "nop", "switch", "activate", "deactivate"][r.below(6)];
            let prio = [-1, 0, 0, 1, 5][r.below(5)];
            let (a, b) = match kind {
                "switch" => (r.below(2), 0),
                "activate" => (r.below(nscripts), r.below(2)),
                "deactivate" => (r.below(nscripts), 0),
                _ => (0, 0),
            };
            s.blocks.push((sc, kind, prio, a, b));
        }
        let last = s.blocks.len();
        for _ in 0..1 + r.below(2) {
            s.links
                .push((sc, None, first + r.below(n), [0, 0, 1, 2][r.below(4)]));
        }
        for b in first..last {
            let outs = if s.blocks[b].1 == "switch" { 2 } else { 1 };
            for o in 0..outs {
                if r.below(3) == 0 {
                    continue;
                }
                let delay = [0u16, 0, 1, 2, 3][r.below(5)];
                // a zero-delay link only goes forward, so no graph spins within one frame
                let to = if delay == 0 {
                    if b + 1 >= last {
                        continue;
                    }
                    b + 1 + r.below(last - b - 1)
                } else {
                    first + r.below(n)
                };
                s.links.push((sc, Some((b, o)), to, delay));
            }
        }
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
        "the oracle failed on:\n{}",
        spec.text()
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn random_trivial_graphs_trace_as_the_c_runtime_does() {
    let Some(bin) = std::env::var_os("OPENBALLANCE_ORACLE") else {
        eprintln!(
            "OPENBALLANCE_ORACLE not set: skipped (cmake --build target/ck-oracle --target ck_oracle)"
        );
        return;
    };
    let bin = std::path::PathBuf::from(bin);
    let cases: u64 = std::env::var("ORACLE_CASES")
        .ok()
        .and_then(|c| c.parse().ok())
        .unwrap_or(500);
    let dir = std::env::temp_dir().join(format!("drahtseilakt-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("graph.txt");
    for seed in 0..cases {
        let spec = random_spec(seed);
        let ours = spec.run();
        let theirs = oracle(&bin, &file, &spec);
        if ours == theirs {
            continue;
        }
        {
            let at = ours
                .lines()
                .zip(theirs.lines())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            panic!(
                "seed {seed}: traces differ at line {at}\n--- graph\n{}--- ours\n{ours}--- C\n{theirs}",
                spec.text()
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_spec_runs_in_our_runtime_without_the_oracle() {
    // the generator and the runner themselves, everywhere (no C needed): a deferred activation shows up
    let spec = Spec {
        objects: 2,
        scripts: vec![(0, true), (1, false)],
        blocks: vec![(0, "activate", 0, 1, 1), (1, "nop", 0, 0, 0)],
        links: vec![(0, None, 0, 0), (1, None, 1, 0)],
        frames: 3,
    };
    assert_eq!(
        spec.run(),
        "frame 1\nrun B0\nend\nframe 2\nrun B1\nend\nframe 3\nend\n"
    );
    for seed in 0..50 {
        random_spec(seed).run();
    }
}
