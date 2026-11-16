// THIRD-PARTY.md lists every crate in Cargo.lock that the game can be built from, each with a licence compatible with
// GPL-3.0. A new or updated dependency fails this test until `python3 -I tools/third_party.py` has been run (which
// reads the licence from the crate) and the licence is one the GPL allows. Data-free: reads only the repository.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

const BEGIN: &str = "<!-- third-party:crates:begin";
const END: &str = "<!-- third-party:crates:end -->";

/// Licences (SPDX ids) a GPL-3.0-only program may include; kept in step with ALLOWED in tools/third_party.py.
const ALLOWED: &[&str] = &[
    "0BSD",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-1-Clause",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "CC0-1.0",
    "CDLA-Permissive-2.0",
    "GPL-2.0-or-later",
    "GPL-3.0",
    "GPL-3.0-only",
    "GPL-3.0-or-later",
    "ISC",
    "LGPL-2.1-or-later",
    "LGPL-3.0-only",
    "LGPL-3.0-or-later",
    "MIT",
    "MIT-0",
    "MPL-2.0",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "Unlicense",
    "Zlib",
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Package {
    name: String,
    version: String,
    registry: bool,
    deps: Vec<String>,
}

/// The packages of a Cargo.lock (version 3 or 4 layout: `[[package]]` blocks, one key per line).
fn lock_packages(text: &str) -> Vec<Package> {
    let mut out: Vec<Package> = Vec::new();
    let mut in_deps = false;
    for line in text.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            out.push(Package {
                name: String::new(),
                version: String::new(),
                registry: false,
                deps: Vec::new(),
            });
            in_deps = false;
            continue;
        }
        let Some(p) = out.last_mut() else { continue };
        let quoted = |s: &str| s.split('"').nth(1).unwrap_or("").to_owned();
        if in_deps {
            if line.starts_with(']') {
                in_deps = false;
            } else {
                p.deps.push(quoted(line));
            }
        } else if let Some(v) = line.strip_prefix("name = ") {
            p.name = quoted(v);
        } else if let Some(v) = line.strip_prefix("version = ") {
            p.version = quoted(v);
        } else if line.starts_with("source = ") {
            p.registry = true;
        } else if line.starts_with("dependencies = [") {
            in_deps = !line.ends_with(']');
        }
    }
    out
}

/// Every registry crate reachable from the game's package, over all of Cargo.lock's edges (all platforms, build
/// scripts included), leaving out the workspace's test-only crate.
fn reachable() -> BTreeSet<(String, String)> {
    let text = std::fs::read_to_string(repo().join("Cargo.lock")).expect("Cargo.lock");
    let pkgs = lock_packages(&text);
    let mut by_name: HashMap<&str, Vec<&Package>> = HashMap::new();
    for p in &pkgs {
        by_name.entry(&p.name).or_default().push(p);
    }
    let find = |dep: &str| -> &Package {
        let mut parts = dep.split(' ');
        let name = parts.next().unwrap_or("");
        let cands = by_name
            .get(name)
            .unwrap_or_else(|| panic!("{dep} not in Cargo.lock"));
        match parts.next() {
            Some(v) => cands
                .iter()
                .find(|p| p.version == v)
                .unwrap_or_else(|| panic!("{dep} not in Cargo.lock")),
            None => cands.first().expect("one package"),
        }
    };
    let mut seen = BTreeSet::new();
    let mut out = BTreeSet::new();
    let mut stack = vec![find("drahtseilakt")];
    while let Some(p) = stack.pop() {
        if !seen.insert((p.name.clone(), p.version.clone())) {
            continue;
        }
        if p.registry {
            out.insert((p.name.clone(), p.version.clone()));
        }
        for d in &p.deps {
            let dp = find(d);
            if dp.name != "fuzzkit" {
                stack.push(dp);
            }
        }
    }
    out
}

/// The rows of THIRD-PARTY.md's generated tables: (name, version) -> licence.
fn listed() -> HashMap<(String, String), String> {
    let text = std::fs::read_to_string(repo().join("THIRD-PARTY.md"))
        .expect("THIRD-PARTY.md at the repository root");
    let start = text.find(BEGIN).expect("the begin marker");
    let end = text.find(END).expect("the end marker");
    let part = text.get(start..end).expect("markers in order");
    let mut out = HashMap::new();
    for line in part.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // "| name | version | licence |" splits into ["", name, version, licence, ""]
        if cells.len() != 5 || cells.get(1) == Some(&"Crate") || cells.get(1) == Some(&"---") {
            continue;
        }
        let licence = cells[3].trim_end_matches('†').trim().to_owned();
        out.insert((cells[1].to_owned(), cells[2].to_owned()), licence);
    }
    out
}

/// SPDX tokens; `/` is the old spelling of OR, `X WITH Y` is one licence.
fn tokens(expr: &str) -> Vec<String> {
    let spaced = expr
        .replace('/', " OR ")
        .replace('(', " ( ")
        .replace(')', " ) ");
    let mut out: Vec<String> = Vec::new();
    let mut with = false;
    for t in spaced.split_whitespace() {
        if t == "WITH" {
            with = true;
        } else if with {
            if let Some(last) = out.last_mut() {
                *last = format!("{last} WITH {t}");
            }
            with = false;
        } else {
            out.push(t.to_owned());
        }
    }
    out
}

/// Whether the expression can be met with ALLOWED licences (OR: either side, AND: both).
fn compatible(expr: &str) -> bool {
    fn or_expr(t: &[String], i: &mut usize) -> Option<bool> {
        let mut v = and_expr(t, i)?;
        while t.get(*i).map(String::as_str) == Some("OR") {
            *i += 1;
            v = and_expr(t, i)? || v;
        }
        Some(v)
    }
    fn and_expr(t: &[String], i: &mut usize) -> Option<bool> {
        let mut v = atom(t, i)?;
        while t.get(*i).map(String::as_str) == Some("AND") {
            *i += 1;
            v = atom(t, i)? && v;
        }
        Some(v)
    }
    fn atom(t: &[String], i: &mut usize) -> Option<bool> {
        let tok = t.get(*i)?.as_str();
        *i += 1;
        match tok {
            "(" => {
                let v = or_expr(t, i)?;
                (t.get(*i).map(String::as_str) == Some(")")).then(|| *i += 1)?;
                Some(v)
            }
            ")" | "AND" | "OR" => None,
            id => Some(ALLOWED.contains(&id)),
        }
    }
    let t = tokens(expr);
    let mut i = 0;
    matches!(or_expr(&t, &mut i), Some(true)) && i == t.len()
}

#[test]
fn spdx_evaluation() {
    assert!(compatible("MIT OR Apache-2.0"));
    assert!(compatible("MIT/Apache-2.0"));
    assert!(compatible("(MIT OR Apache-2.0) AND Unicode-3.0"));
    assert!(compatible(
        "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT"
    ));
    assert!(compatible("Apache-2.0 OR GPL-2.0-only"));
    assert!(!compatible("GPL-2.0-only"));
    assert!(!compatible(
        "(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0"
    ));
    assert!(!compatible(""));
    assert!(!compatible("MIT OR"));
}

#[test]
fn every_dependency_is_listed_with_a_compatible_licence() {
    let lock = reachable();
    let listed = listed();
    let keys: BTreeSet<(String, String)> = listed.keys().cloned().collect();
    let missing: Vec<_> = lock.difference(&keys).collect();
    let stale: Vec<_> = keys.difference(&lock).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "THIRD-PARTY.md is out of date with Cargo.lock (run python3 -I tools/third_party.py): \
         missing {missing:?}, no longer in the lock {stale:?}"
    );
    let bad: Vec<_> = listed
        .iter()
        .filter(|(_, licence)| !compatible(licence))
        .map(|((n, v), l)| format!("{n} {v}: '{l}'"))
        .collect();
    assert!(
        bad.is_empty(),
        "crates without a licence or with one not compatible with GPL-3.0: {bad:?}"
    );
}
