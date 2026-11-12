// The census of behaviour graphs over a set of files: how many files carry graphs, how many scripts, blocks and
// links, the link delays, and every block kind and operation met, with where the registry stands on each (its
// library and standing, or unknown). `script-dump --census` prints it as CSV; the counts tell which kinds a port
// stage should take next (docs/drahtseilakt/scripts.md, "Stage 0").

use crate::blocks::{self, Standing};
use crate::{Graph, Guid, Node};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What one kind (a block prototype or an operation) was seen as: a name, and in how many files and places.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    pub name: String,
    pub files: usize,
    pub instances: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    pub files: usize,
    pub with_graphs: usize,
    pub scripts: usize,
    pub graphs: usize,
    pub blocks: usize,
    pub links: usize,
    /// Links by delay: 0, 1, 2, more.
    pub delays: [usize; 4],
    /// Behaviours with a priority other than 0, and objects with one.
    pub behaviour_priorities: usize,
    pub object_priorities: usize,
    /// Scripts and objects the scene starts deactivated.
    pub start_off: usize,
    pub kinds: BTreeMap<(u32, u32), Seen>,
    pub ops: BTreeMap<(u32, u32), Seen>,
    /// Every operation signature met (operation, input 1, input 2, output types), in a graph's own operations
    /// and in Op blocks.
    pub signatures: BTreeMap<Signature, Seen>,
    /// Per file with graphs: (file, block kinds the registry lacks, signatures the table lacks, kinds it runs
    /// only in part).
    pub maps: Vec<MapRow>,
}

/// An operation signature: the operation and its input and output types, as the table looks it up.
pub type Signature = (Guid, Guid, Guid, Guid);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapRow {
    pub file: String,
    pub missing: Vec<String>,
    pub ops_missing: Vec<String>,
    pub partial: Vec<String>,
}

/// The signatures of a graph's operations: its own operations and its Op blocks (the operation's GUID in locals 1
/// and 2, the types of pins 0 and 1 and of pout 0), with the name the file gives each.
pub fn signatures(g: &Graph) -> Vec<(Signature, String)> {
    let ty = |p: Option<&crate::Id>| -> Guid {
        p.and_then(|&p| g.param(p))
            .map_or(crate::types::NONE, |p| p.guid)
    };
    let mut out = Vec::new();
    for (i, n) in g.nodes.iter().enumerate() {
        match n {
            Node::Op(o) => out.push((
                (
                    o.guid,
                    ty(o.inputs[0].as_ref()),
                    ty(o.inputs[1].as_ref()),
                    ty(o.output.as_ref()),
                ),
                g.name(i).to_string(),
            )),
            Node::Block(b) if b.proto == Some(blocks::proto::OP) => {
                let local = |k: usize| {
                    b.locals
                        .get(k)
                        .and_then(|&l| g.param(l))
                        .map_or(0, |p| p.value.as_int() as u32)
                };
                out.push((
                    (
                        Guid(local(1), local(2)),
                        ty(b.pins.first()),
                        ty(b.pins.get(1)),
                        ty(b.pouts.first()),
                    ),
                    b.name.clone(),
                ));
            }
            _ => {}
        }
    }
    out
}

fn count(map: &mut BTreeMap<(u32, u32), Seen>, g: Guid, name: &str, file: &mut Vec<(u32, u32)>) {
    let e = map.entry((g.0, g.1)).or_default();
    if e.name.is_empty() {
        e.name = name.to_string();
    }
    e.instances = e.instances.saturating_add(1);
    if !file.contains(&(g.0, g.1)) {
        file.push((g.0, g.1));
        e.files = e.files.saturating_add(1);
    }
}

/// How deep a census walks into folders, and how many files it reads at most.
const MAX_WALK: usize = 8;
const MAX_FILES: usize = 100_000;

/// The Virtools files (.nmo, .cmo) under a path, or the path itself, sorted; links are not followed.
pub fn files_under(path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(path, 0, &mut out);
    out
}

fn walk(path: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return;
    };
    if meta.is_file() {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase());
        if matches!(ext.as_deref(), Some("nmo" | "cmo")) && out.len() < MAX_FILES {
            out.push(path.to_path_buf());
        }
        return;
    }
    if !meta.is_dir() || depth >= MAX_WALK {
        return;
    }
    let Ok(rd) = std::fs::read_dir(path) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for e in entries {
        walk(&e, depth.saturating_add(1), out);
    }
}

impl Census {
    /// Counts every Virtools file under a path; returns what could not be read.
    pub fn add_files(&mut self, path: &Path) -> Vec<String> {
        let mut errors = Vec::new();
        for f in files_under(path) {
            let doc = std::fs::read(&f)
                .map_err(|e| e.to_string())
                .and_then(|b| nmo::read(&b).map_err(|e| e.to_string()))
                .and_then(|file| nmo::Document::parse(&file).map_err(|e| e.to_string()));
            match doc {
                Ok(d) => {
                    let g = Graph::from_document(&d, &|_| crate::ObjKind::Other);
                    self.add(&g);
                    if !g.scripts.is_empty() {
                        self.maps.push(map_row(&f.to_string_lossy(), &g));
                    }
                }
                Err(e) => errors.push(format!("{}: {e}", f.display())),
            }
        }
        errors
    }

    /// Counts one file's graphs.
    pub fn add(&mut self, g: &Graph) {
        self.files = self.files.saturating_add(1);
        if g.scripts.is_empty() {
            return;
        }
        self.with_graphs = self.with_graphs.saturating_add(1);
        self.scripts = self.scripts.saturating_add(g.scripts.len());
        self.object_priorities = self.object_priorities.saturating_add(g.priorities.len());
        self.start_off = self.start_off.saturating_add(g.start_off.len());
        let (mut kinds, mut ops) = (Vec::new(), Vec::new());
        for n in &g.nodes {
            match n {
                Node::Block(b) => {
                    if b.priority != 0 {
                        self.behaviour_priorities = self.behaviour_priorities.saturating_add(1);
                    }
                    match b.proto {
                        Some(p) => {
                            self.blocks = self.blocks.saturating_add(1);
                            count(&mut self.kinds, p, &b.name, &mut kinds);
                        }
                        None => self.graphs = self.graphs.saturating_add(1),
                    }
                }
                Node::Link(l) => {
                    self.links = self.links.saturating_add(1);
                    let slot = usize::from(l.delay.min(3));
                    if let Some(d) = self.delays.get_mut(slot) {
                        *d = d.saturating_add(1);
                    }
                }
                Node::Op(o) => count(&mut self.ops, o.guid, "", &mut ops),
                _ => {}
            }
        }
        let mut sigs = Vec::new();
        for (sig, name) in signatures(g) {
            let e = self.signatures.entry(sig).or_default();
            // an Op block is named "Op" unless its author renamed it; a graph's own operation by the operation
            if e.name.is_empty() || (e.name == "Op" && !name.is_empty()) {
                e.name = name;
            }
            e.instances = e.instances.saturating_add(1);
            if !sigs.contains(&sig) {
                sigs.push(sig);
                e.files = e.files.saturating_add(1);
            }
        }
    }

    /// The census as CSV: a summary block, then one row per block kind (most used first) and one per operation.
    pub fn csv(&self) -> String {
        let mut out = String::new();
        let mut line = |s: String| {
            out.push_str(&s);
            out.push('\n');
        };
        line(format!(
            "summary,files {},with graphs {},scripts {},graphs {},blocks {},links {},delay 0 {},delay 1 {},delay 2 {},delay >2 {},behaviour priorities {},object priorities {},start deactivated {}",
            self.files,
            self.with_graphs,
            self.scripts,
            self.graphs,
            self.blocks,
            self.links,
            self.delays[0],
            self.delays[1],
            self.delays[2],
            self.delays[3],
            self.behaviour_priorities,
            self.object_priorities,
            self.start_off
        ));
        line("kind,name,guid,library,standing,files,instances".into());
        let mut kinds: Vec<_> = self.kinds.iter().collect();
        kinds.sort_by(|a, b| b.1.instances.cmp(&a.1.instances).then(a.0.cmp(b.0)));
        for (&(g0, g1), seen) in kinds {
            let decl = blocks::find(Guid(g0, g1));
            let (name, library, standing) = match decl {
                Some(p) => (
                    p.name.to_string(),
                    p.library.dll(),
                    match p.standing {
                        Standing::Full => "full",
                        Standing::Partial => "partial",
                        Standing::NoOp => "no-op",
                    },
                ),
                None => (seen.name.clone(), "?", "unknown"),
            };
            line(format!(
                "block,{},{g0:08x}:{g1:08x},{library},{standing},{},{}",
                name.replace(',', ";"),
                seen.files,
                seen.instances
            ));
        }
        for (&(g0, g1), seen) in &self.ops {
            let known = if crate::ops::known(Guid(g0, g1)) {
                "known"
            } else {
                "unknown"
            };
            line(format!(
                "op,,{g0:08x}:{g1:08x},,{known},{},{}",
                seen.files, seen.instances
            ));
        }
        line("signature,name,op,in1,in2,out,standing,files,instances".into());
        for (&(op, a, b, r), seen) in &self.signatures {
            // known: by which row (its source and signature) the table answers
            let standing = match crate::ops::find(op, r, a, b) {
                Some((row, swap)) => format!(
                    "{} {}<-{}+{}{}",
                    match row.source {
                        crate::ops::Source::C => "C",
                        crate::ops::Source::Name => "name",
                        crate::ops::Source::Dll => "dll",
                    },
                    crate::types::name(row.res),
                    crate::types::name(row.p1),
                    crate::types::name(row.p2),
                    if swap { " swapped" } else { "" }
                ),
                None => "unknown".to_string(),
            };
            line(format!(
                "signature,{},{op},{},{},{},{standing},{},{}",
                seen.name.replace(',', ";"),
                crate::types::name(a),
                crate::types::name(b),
                crate::types::name(r),
                seen.files,
                seen.instances
            ));
        }
        let full = self
            .maps
            .iter()
            .filter(|m| m.missing.is_empty() && m.ops_missing.is_empty() && m.partial.is_empty())
            .count();
        let known = self
            .maps
            .iter()
            .filter(|m| m.missing.is_empty() && m.ops_missing.is_empty())
            .count();
        line(format!(
            "maps,with graphs {},every block and operation known {known},full effect {full}",
            self.maps.len()
        ));
        for m in &self.maps {
            line(format!(
                "map,{},{},{},{}",
                m.file.replace(',', ";"),
                m.missing.join("; ").replace(',', ";"),
                m.ops_missing.join("; ").replace(',', ";"),
                m.partial.join("; ").replace(',', ";")
            ));
        }
        out
    }
}

/// One file's standing: the block kinds the registry lacks or runs in part, and the operation signatures the
/// table lacks.
pub fn map_row(file: &str, g: &Graph) -> MapRow {
    let mut row = MapRow {
        file: file.to_string(),
        ..MapRow::default()
    };
    for (guid, name, _) in g.prototypes() {
        match blocks::find(guid) {
            None => row.missing.push(name),
            Some(p) if p.standing == Standing::Partial => row.partial.push(p.name.to_string()),
            Some(_) => {}
        }
    }
    for ((op, a, b, r), name) in signatures(g) {
        if crate::ops::find(op, r, a, b).is_none() {
            let s = format!("{name} {op}");
            if !row.ops_missing.contains(&s) {
                row.ops_missing.push(s);
            }
        }
    }
    row.missing.sort();
    row.partial.sort();
    row.ops_missing.sort();
    row
}
