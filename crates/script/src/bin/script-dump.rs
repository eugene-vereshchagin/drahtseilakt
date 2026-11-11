// script-dump: prints the behaviour graphs of a .nmo as a tree, for reading what a map's scripts do: each
// script with its owner, each graph's blocks (prototype, target, pins with the values they read, outputs),
// and the links between their inputs and outputs with their delays. A reading aid, not part of the game.
// --census FILE|DIR...: the census of every .nmo/.cmo given or under the folders (CSV: block kinds and operations with
// the registry's standing). --run N FILE: runs the file's scripts N ticks alone (no level, no engine) and prints the
// runtime's trace.

use script::{Graph, Id, Node, ObjKind, Value};
use std::io::Write as _;
use std::process::ExitCode;

/// How deep a graph's boxes are printed (a hostile file may nest without end).
const MAX_DEPTH: usize = 32;
/// How far an input's source chain is followed.
const MAX_CHAIN: usize = 16;

fn short(v: &Value) -> String {
    match v {
        Value::Raw(b) => format!("raw{:02x?}", b.get(..b.len().min(16)).unwrap_or_default()),
        Value::Matrix(_) => "matrix".into(),
        other => format!("{other:?}"),
    }
}

/// What an input parameter reads: its chain of sources down to a value or an operation.
fn source(g: &Graph, p: Id) -> String {
    let mut at = p;
    for _ in 0..MAX_CHAIN {
        let Some(q) = g.param(at) else {
            return format!("#{at}?");
        };
        if let Some(s) = q.source {
            at = s;
            continue;
        }
        let what = match &q.value {
            Value::Obj(Some(o)) => format!("obj #{o} {:?}", g.name(*o)),
            v => short(v),
        };
        let op = match q.op.and_then(|o| g.nodes.get(o)) {
            Some(Node::Op(o)) => format!(" = op {} ({:?}, {:?})", o.guid, o.inputs[0], o.inputs[1]),
            _ => String::new(),
        };
        return format!("#{at} {:?} {what}{op}", q.name);
    }
    "...".into()
}

fn io_name(g: &Graph, io: Id) -> String {
    let owner = g.io(io).and_then(|i| i.owner);
    let Some(owner) = owner else {
        return format!("io#{io}");
    };
    let Some(b) = g.block(owner) else {
        return format!("io#{io}");
    };
    if let Some(k) = b.inputs.iter().position(|&x| x == io) {
        return format!("#{owner} {:?} in{k} {:?}", b.name, g.name(io));
    }
    if let Some(k) = b.outputs.iter().position(|&x| x == io) {
        return format!("#{owner} {:?} out{k} {:?}", b.name, g.name(io));
    }
    format!("io#{io}")
}

fn block(g: &Graph, id: Id, depth: usize) {
    let pad = "  ".repeat(depth.saturating_add(1));
    let Some(b) = g.block(id) else { return };
    let proto = b.proto.map_or("graph".to_string(), |p| p.to_string());
    let known = b.proto.is_none_or(script::blocks::known);
    println!(
        "{pad}#{id} {:?} [{proto}]{} prio {} flags {:#x}",
        b.name,
        if known { "" } else { " UNKNOWN" },
        b.priority,
        b.flags
    );
    if let Some(t) = b.target {
        println!("{pad}  target <- {}", source(g, t));
    }
    for (k, &io) in b.inputs.iter().enumerate() {
        println!("{pad}  in{k} {:?}", g.name(io));
    }
    for (k, &io) in b.outputs.iter().enumerate() {
        println!("{pad}  out{k} {:?}", g.name(io));
    }
    for (k, &p) in b.pins.iter().enumerate() {
        println!("{pad}  pin{k} {:?} <- {}", g.name(p), source(g, p));
    }
    for (k, &p) in b.pouts.iter().enumerate() {
        let dests = g
            .param(p)
            .map(|q| q.destinations.clone())
            .unwrap_or_default();
        println!("{pad}  pout{k} #{p} {:?} -> {dests:?}", g.name(p));
    }
    if b.proto.is_some() {
        for (k, &p) in b.locals.iter().enumerate() {
            let v = g.param(p).map(|q| short(&q.value)).unwrap_or_default();
            println!("{pad}  local{k} #{p} {:?} = {v}", g.name(p));
        }
    }
    for &l in &b.links {
        if let Some(link) = g.link(l) {
            println!(
                "{pad}  link {} => {}{}",
                io_name(g, link.src),
                io_name(g, link.dst),
                if link.delay > 0 {
                    format!(" (delay {})", link.delay)
                } else {
                    String::new()
                }
            );
        }
    }
    if depth < MAX_DEPTH {
        for &s in &b.sub {
            block(g, s, depth.saturating_add(1));
        }
    }
}

fn load(path: &std::path::Path) -> Result<nmo::Document, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let file = nmo::read(&bytes).map_err(|e| e.to_string())?;
    nmo::Document::parse(&file).map_err(|e| e.to_string())
}

fn census(paths: &[String]) -> ExitCode {
    let mut c = script::census::Census::default();
    for p in paths {
        for e in c.add_files(std::path::Path::new(p)) {
            eprintln!("{e}");
        }
    }
    print!("{}", c.csv());
    ExitCode::SUCCESS
}

fn run(ticks: &str, path: &str) -> ExitCode {
    let Ok(ticks) = ticks.parse::<u64>() else {
        eprintln!("--run needs a tick count");
        return ExitCode::from(2);
    };
    let doc = match load(std::path::Path::new(path)) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let g = std::sync::Arc::new(Graph::from_document(&doc, &|_| ObjKind::Other));
    let mut rt = script::Runtime::new(g);
    rt.set_trace(true);
    let mut host = script::Quiet;
    let mut out = std::io::stdout().lock();
    for _ in 0..ticks.min(1_000_000) {
        rt.tick(&mut host);
        for l in rt.take_trace() {
            // a closed pipe (| head) ends the run
            if writeln!(out, "{l}").is_err() {
                return ExitCode::SUCCESS;
            }
        }
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--census") => return census(args.get(1..).unwrap_or_default()),
        Some("--run") => {
            return match (args.get(1), args.get(2)) {
                (Some(n), Some(f)) => run(n, f),
                _ => {
                    eprintln!("usage: script-dump --run TICKS FILE.nmo");
                    ExitCode::from(2)
                }
            };
        }
        _ => {}
    }
    if args.is_empty() {
        eprintln!("usage: script-dump FILE.nmo... | --census FILE|DIR... | --run TICKS FILE.nmo");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for path in &args {
        let doc = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                let file = nmo::read(&bytes).map_err(|e| e.to_string())?;
                nmo::Document::parse(&file).map_err(|e| e.to_string())
            });
        let doc = match doc {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{path}: {e}");
                failed = true;
                continue;
            }
        };
        let g = Graph::from_document(&doc, &|_| ObjKind::Other);
        println!("{path}: {} scripts", g.scripts.len());
        for &s in &g.scripts {
            let owner = g.block(s).and_then(|b| b.owner);
            println!(
                "script #{s} {:?} on {}",
                g.name(s),
                owner.map_or("nothing".to_string(), |o| format!("#{o} {:?}", g.name(o)))
            );
            block(&g, s, 0);
        }
        for a in &g.arrays {
            println!("array {:?}: {:?}", a.name, a.columns);
            for r in &a.rows {
                println!("  {}", r.iter().map(short).collect::<Vec<_>>().join(" | "));
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
