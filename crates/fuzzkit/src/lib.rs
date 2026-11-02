// fuzzkit: a small mutation fuzzer that runs on stable Rust with no dependencies, for the parsers' `#[ignore]` fuzz
// tests (docs/drahtseilakt/fuzzing.md). The cargo-fuzz targets in fuzz/ drive the same entry points under libFuzzer
// on nightly; this one runs anywhere the tests run, offline.
//
// How a run goes: the test calls `fuzz`, which becomes a supervisor. It starts the same test binary again as a child
// (the same test, FUZZ_CHILD set) that does the fuzzing; inputs are made from a corpus (seeds) by a deterministic
// xorshift generator, so input number `i` is a pure function of the seed, the corpus and `i` and a crash can be made
// again from its number. In the child:
//   - a panic is caught (catch_unwind), its place is the signature; the first input per signature is written to
//     fuzz/artifacts/<target>/crash-<hash>.bin and minimised in-process to crash-<hash>.min.bin;
//   - a watchdog thread ends the child when one input runs longer than the deadline (hang-*.bin) or the process's
//     resident memory passes the cap (oom-*.bin), writing the input first;
//   - an abort (an allocation failure under the address-space limit set with prlimit, a stack overflow) kills the
//     child without a word: the supervisor then runs the last stretch again in trace mode, which writes every input
//     to a file before running it, and keeps the one that killed it (abort-*.bin).
// Hangs, ooms and aborts are minimised by the supervisor, one child run per candidate.
//
// Settings (environment): FUZZ_SECS (time budget, default 10), FUZZ_SEED (default 1), FUZZ_TIMEOUT_MS (one input's
// deadline, default 10000), FUZZ_RSS_MB (memory cap, default per target), FUZZ_NO_REAL (no seeds from the player's
// files), FUZZ_REPRO=<file> (run one input and say what it does), FUZZ_DIR (instead of <workspace>/fuzz),
// FUZZ_DUMP_CORPUS (write the seeds to fuzz/corpus/<target>/ for cargo-fuzz, and stop).

// Test tooling, not a parser of untrusted input: the hostile-input lints are for the game's code.
#![allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod mutate;

pub use mutate::Rng;

use std::io::{BufRead, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How a target's inputs are mutated: as bytes, or also as text (lines, words and numbers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Binary,
    Text,
}

/// What one fuzz target is.
pub struct Target {
    /// The name its artifacts and regressions are kept under (fuzz/artifacts/<name>, fuzz/regressions/<name>).
    pub name: &'static str,
    /// The test's path as libtest knows it (`fuzz_tests::fuzz_replay`), from `test_name!`, to start it as a child.
    pub test: String,
    pub kind: Kind,
    /// The largest input made.
    pub max_len: usize,
    /// The resident memory a child may reach, in MiB (FUZZ_RSS_MB overrides it).
    pub rss_mb: u64,
    /// Words the text mutations insert besides those found in the seeds (keywords of the format).
    pub dict: &'static [&'static str],
}

impl Target {
    pub fn new(name: &'static str, test: String, kind: Kind) -> Target {
        Target { name, test, kind, max_len: 1 << 20, rss_mb: 512, dict: &[] }
    }
}

/// The libtest path of a test function in the calling module: `test_name!("fuzz_replay")`.
#[macro_export]
macro_rules! test_name {
    ($f:literal) => {
        match module_path!().split_once("::") {
            Some((_, rest)) => format!("{rest}::{}", $f),
            None => $f.to_string(),
        }
    };
}

/// fuzz/ in the workspace (or FUZZ_DIR).
pub fn root() -> PathBuf {
    match std::env::var_os("FUZZ_DIR") {
        Some(d) => PathBuf::from(d),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz"),
    }
}

fn artifacts(name: &str) -> PathBuf {
    root().join("artifacts").join(name)
}

static SCRATCH: OnceLock<PathBuf> = OnceLock::new();

/// A folder of this process for targets that must read from files (disc images, level folders): under the target's
/// artifacts (on disk, never /tmp), removed when the run ends.
pub fn scratch() -> PathBuf {
    SCRATCH
        .get_or_init(|| {
            let d = artifacts("work").join(format!("p{}", std::process::id()));
            let _ = std::fs::create_dir_all(&d);
            d
        })
        .clone()
}

fn drop_scratch() {
    if let Some(d) = SCRATCH.get() {
        let _ = std::fs::remove_dir_all(d);
    }
}

pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

// ---------------------------------------------------------------------------------------------------------------
// Seeds

/// The folders the player's files are read from as seeds (never copied into the repository): OPENBALLANCE_DATA,
/// OPENBALLANCE_MAPS and the profile's original and community level folders, those that exist. None with FUZZ_NO_REAL.
pub fn real_roots() -> Vec<PathBuf> {
    if std::env::var_os("FUZZ_NO_REAL").is_some() {
        return Vec::new();
    }
    let mut out: Vec<PathBuf> = ["OPENBALLANCE_DATA", "OPENBALLANCE_MAPS"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    if let Some(home) = std::env::var_os("HOME") {
        let levels = PathBuf::from(home).join(".local/share/drahtseilakt/levels");
        out.push(levels.join("original"));
        out.push(levels.join("community"));
    }
    out.retain(|p| p.is_dir());
    out
}

/// Files under `roots` whose extension is one of `exts` (any case), at most `max_bytes` each; when there are more than
/// `max_files`, an even spread of them. Sorted, so a run is the same each time.
pub fn files_under(roots: &[PathBuf], exts: &[&str], max_files: usize, max_bytes: u64) -> Vec<PathBuf> {
    fn walk(d: &Path, depth: usize, exts: &[&str], max_bytes: u64, out: &mut Vec<PathBuf>) {
        if depth > 8 || out.len() > 100_000 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(d) else { return };
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let Ok(t) = e.file_type() else { continue };
            let p = e.path();
            if t.is_dir() {
                walk(&p, depth + 1, exts, max_bytes, out);
            } else if t.is_file() {
                let ext = p.extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                let name = p.file_name().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                let wanted = exts.iter().any(|x| x.eq_ignore_ascii_case(&ext) || x.eq_ignore_ascii_case(&name));
                if wanted && e.metadata().is_ok_and(|m| m.len() <= max_bytes) {
                    out.push(p);
                }
            }
        }
    }
    let mut all = Vec::new();
    for r in roots {
        walk(r, 0, exts, max_bytes, &mut all);
    }
    if all.len() > max_files && max_files > 0 {
        let step = all.len() as f64 / max_files as f64;
        all = (0..max_files).filter_map(|i| all.get((i as f64 * step) as usize).cloned()).collect();
    }
    all
}

/// The contents of `files_under(..)`.
pub fn read_all(paths: &[PathBuf]) -> Vec<Vec<u8>> {
    paths.iter().filter_map(|p| std::fs::read(p).ok()).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// Running one input

static LAST_PANIC: Mutex<Option<(String, String)>> = Mutex::new(None);

fn quiet_panics() {
    std::panic::set_hook(Box::new(|info| {
        let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        if let Ok(mut l) = LAST_PANIC.lock() {
            *l = Some((place, msg));
        }
    }));
}

/// Runs one input; `Some((place, message))` when it panicked.
fn run_one(run: &dyn Fn(&[u8]), input: &[u8]) -> Option<(String, String)> {
    if let Ok(mut l) = LAST_PANIC.lock() {
        *l = None;
    }
    match catch_unwind(AssertUnwindSafe(|| run(input))) {
        Ok(()) => None,
        Err(_) => Some(LAST_PANIC.lock().ok().and_then(|mut l| l.take()).unwrap_or_default()),
    }
}

/// What the watchdog sees: the input running, since when, and which.
#[derive(Default)]
struct Current {
    input: Option<Arc<Vec<u8>>>,
    since: Option<Instant>,
    iter: u64,
}

fn rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok()))
        .map_or(0, |pages| pages * 4096 / (1024 * 1024))
}

fn write_finding(name: &str, prefix: &str, input: &[u8], note: &str) -> PathBuf {
    let dir = artifacts(name);
    let _ = std::fs::create_dir_all(&dir);
    let stem = format!("{prefix}-{:016x}", fnv(input));
    let path = dir.join(format!("{stem}.bin"));
    let _ = std::fs::write(&path, input);
    let _ = std::fs::write(dir.join(format!("{stem}.txt")), note);
    path
}

fn watchdog(name: Option<&'static str>, current: Arc<Mutex<Current>>, timeout: Duration, rss_cap: u64) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(20));
            let Ok(c) = current.lock() else { continue };
            let over_time = c.since.is_some_and(|s| s.elapsed() > timeout);
            let rss = rss_mb();
            if !over_time && rss <= rss_cap {
                continue;
            }
            let (kind, code) = if over_time { ("hang", 3) } else { ("oom", 4) };
            let input = c.input.clone().unwrap_or_default();
            let note = format!("{kind} at input {} (rss {rss} MiB, cap {rss_cap}, deadline {timeout:?})\n", c.iter);
            // a repro run (name None) only says what happened; the fuzzing run keeps the input
            let path = name.map(|n| write_finding(n, kind, &input, &note)).unwrap_or_default();
            println!("FUZZ!{kind} {} {}", c.iter, path.display());
            let _ = std::io::stdout().flush();
            std::process::exit(code);
        }
    });
}

/// Smaller inputs that keep `still(input)` true: chunks deleted, halving their size, within `budget` tries and `time`.
pub fn minimize(input: &[u8], mut still: impl FnMut(&[u8]) -> bool, budget: usize, time: Duration) -> Vec<u8> {
    let start = Instant::now();
    let mut cur = input.to_vec();
    let mut tries = 0usize;
    let mut chunk = (cur.len() / 2).max(1);
    let out_of_budget = |tries: usize| tries >= budget || start.elapsed() > time;
    loop {
        let mut i = 0;
        while i < cur.len() && !out_of_budget(tries) {
            let end = (i + chunk).min(cur.len());
            let mut cand = cur[..i].to_vec();
            cand.extend_from_slice(&cur[end..]);
            tries += 1;
            if still(&cand) {
                cur = cand;
            } else {
                i += chunk;
            }
        }
        if chunk == 1 || out_of_budget(tries) {
            break;
        }
        chunk = (chunk / 2).max(1);
    }
    // then bytes made plainer ('0' for text-like bytes, 0 otherwise), which makes a repro easier to read
    for i in 0..cur.len() {
        if out_of_budget(tries) {
            break;
        }
        let plain = if cur[i].is_ascii_graphic() { b'0' } else { 0 };
        if cur[i] == plain {
            continue;
        }
        let mut cand = cur.clone();
        cand[i] = plain;
        tries += 1;
        if still(&cand) {
            cur = cand;
        }
    }
    cur
}

// ---------------------------------------------------------------------------------------------------------------
// The entry point

/// Fuzzes `run` (which must not panic, hang or take unbounded memory on any input) from the corpus `seeds` makes.
/// Panics at the end when this run found something new, so a CI run fails.
pub fn fuzz(target: Target, seeds: impl FnOnce() -> Vec<Vec<u8>>, run: impl Fn(&[u8])) {
    let rss_cap = env_u64("FUZZ_RSS_MB", target.rss_mb);
    let timeout = Duration::from_millis(env_u64("FUZZ_TIMEOUT_MS", 10_000));
    if let Some(file) = std::env::var_os("FUZZ_REPRO") {
        repro(&target, Path::new(&file), &run, timeout, rss_cap);
        return;
    }
    if std::env::var_os("FUZZ_CHILD").is_some() {
        child(&target, seeds, &run, timeout, rss_cap);
        drop_scratch();
        return;
    }
    let found = supervise(&target, rss_cap);
    assert!(found.is_empty(), "{}: new findings: {found:#?}", target.name);
}

/// Replays the committed repros in fuzz/regressions/<name>/ (synthetic inputs only, never game data): none may
/// panic or take more than a few seconds.
pub fn regressions(name: &str, run: impl Fn(&[u8])) {
    let dir = root().join("regressions").join(name);
    let Ok(rd) = std::fs::read_dir(&dir) else { return };
    let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    files.sort();
    let mut bad = Vec::new();
    for f in &files {
        let bytes = std::fs::read(f).unwrap_or_default();
        let start = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(|| run(&bytes)));
        if r.is_err() {
            bad.push(format!("{}: panicked", f.display()));
        } else if start.elapsed() > Duration::from_secs(5) {
            bad.push(format!("{}: took {:?}", f.display(), start.elapsed()));
        }
    }
    drop_scratch();
    assert!(bad.is_empty(), "{bad:#?}");
}

fn repro(_target: &Target, file: &Path, run: &dyn Fn(&[u8]), timeout: Duration, rss_cap: u64) {
    let input = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let current = Arc::new(Mutex::new(Current::default()));
    if let Ok(mut c) = current.lock() {
        c.input = Some(Arc::new(input.clone()));
        c.since = Some(Instant::now());
    }
    watchdog(None, current, timeout, rss_cap);
    quiet_panics();
    let start = Instant::now();
    let result = run_one(run, &input);
    drop_scratch();
    match result {
        Some((place, msg)) => {
            println!("FUZZ!crash {place} {msg}");
            let _ = std::io::stdout().flush();
            std::process::exit(101);
        }
        None => println!("FUZZ= ok in {:?}", start.elapsed()),
    }
}

fn child(target: &Target, seeds: impl FnOnce() -> Vec<Vec<u8>>, run: &dyn Fn(&[u8]), timeout: Duration, rss_cap: u64) {
    let seed = env_u64("FUZZ_SEED", 1);
    let from = env_u64("FUZZ_FROM", 0);
    let to = env_u64("FUZZ_TO", u64::MAX);
    let until = Instant::now() + Duration::from_millis(env_u64("FUZZ_MS", 10_000));
    let trace = std::env::var_os("FUZZ_TRACE").map(PathBuf::from);
    let mut corpus = seeds();
    corpus.retain(|s| s.len() <= target.max_len);
    if corpus.is_empty() {
        corpus.push(Vec::new());
    }
    if std::env::var_os("FUZZ_DUMP_CORPUS").is_some() {
        // the seeds as files, for cargo-fuzz (fuzz/corpus/<name>/, ignored by git: they may be the player's files)
        let dir = root().join("corpus").join(target.name);
        let _ = std::fs::create_dir_all(&dir);
        for s in &corpus {
            let _ = std::fs::write(dir.join(format!("{:016x}", fnv(s))), s);
        }
        println!("FUZZ= {from} 0 0");
        return;
    }
    let dict = mutate::Dict::new(&corpus, target.dict, target.kind);
    println!(
        "FUZZ# corpus {} inputs, {} bytes, {} words",
        corpus.len(),
        corpus.iter().map(Vec::len).sum::<usize>(),
        dict.words.len()
    );
    let current = Arc::new(Mutex::new(Current::default()));
    watchdog(Some(target.name), current.clone(), timeout, rss_cap);
    quiet_panics();
    let mut iter = from;
    let mut execs = 0u64;
    let start = Instant::now();
    while iter < to && Instant::now() < until {
        let input = Arc::new(mutate::input(&corpus, &dict, target.kind, seed, iter, target.max_len));
        if let Some(t) = &trace {
            let _ = std::fs::write(t, input.as_slice());
            println!("FUZZ@ {iter}");
            let _ = std::io::stdout().flush();
        } else if iter.is_multiple_of(64) {
            println!("FUZZ@ {iter}");
            let _ = std::io::stdout().flush();
        }
        if let Ok(mut c) = current.lock() {
            c.input = Some(input.clone());
            c.since = Some(Instant::now());
            c.iter = iter;
        }
        let crashed = run_one(run, &input);
        if let Ok(mut c) = current.lock() {
            c.since = None;
        }
        execs += 1;
        if let Some((place, msg)) = crashed {
            let sig = format!("{place} {msg}");
            let stem = format!("crash-{:016x}", fnv(place.as_bytes()));
            let dir = artifacts(target.name);
            let raw = dir.join(format!("{stem}.bin"));
            if !raw.exists() {
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(&raw, input.as_slice());
                let note = format!("panic at {place}: {msg}\ninput {iter} seed {seed}\n");
                let _ = std::fs::write(dir.join(format!("{stem}.txt")), note);
                let min = minimize(
                    &input,
                    |cand| {
                        if let Ok(mut c) = current.lock() {
                            c.input = Some(Arc::new(cand.to_vec()));
                            c.since = Some(Instant::now());
                        }
                        let same = run_one(run, cand).is_some_and(|(p, _)| p == place);
                        if let Ok(mut c) = current.lock() {
                            c.since = None;
                        }
                        same
                    },
                    20_000,
                    Duration::from_secs(60),
                );
                let _ = std::fs::write(dir.join(format!("{stem}.min.bin")), &min);
                println!("FUZZ!crash {iter} {} {sig}", raw.display());
            }
        }
        iter += 1;
    }
    let secs = start.elapsed().as_secs_f64().max(1e-9);
    println!("FUZZ= {iter} {execs} {:.0}", execs as f64 / secs);
    let _ = std::io::stdout().flush();
}

/// What a child run ended with.
struct Outcome {
    status: Option<ExitStatus>,
    /// The last input number it said it reached, and its last line.
    last: Option<u64>,
    findings: Vec<String>,
    done: Option<(u64, u64)>,
}

fn spawn(target: &Target, env: &[(&str, String)], rss_cap: u64) -> Outcome {
    let exe = std::env::current_exe().expect("the test binary");
    // the address-space limit catches what the resident-memory watch is too slow for (one huge allocation); an
    // allocation failing under it aborts the child, which trace mode then pins down
    let limit = (rss_cap * 2 + 1024) * 1024 * 1024;
    let prlimit = Path::new("/usr/bin/prlimit").exists();
    let mut cmd = if prlimit {
        let mut c = Command::new("/usr/bin/prlimit");
        c.arg(format!("--as={limit}")).arg("--").arg(&exe);
        c
    } else {
        Command::new(&exe)
    };
    cmd.args([target.test.as_str(), "--exact", "--ignored", "--nocapture", "--test-threads=1"])
        .env("FUZZ_CHILD", "1")
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut out = Outcome { status: None, last: None, findings: Vec::new(), done: None };
    let Ok(mut child) = cmd.spawn() else { return out };
    if let Some(stdout) = child.stdout.take() {
        for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(n) = line.strip_prefix("FUZZ@ ") {
                out.last = n.trim().parse().ok();
            } else if let Some(f) = line.strip_prefix("FUZZ!") {
                out.findings.push(f.to_owned());
            } else if let Some(d) = line.strip_prefix("FUZZ= ") {
                let mut w = d.split_whitespace().map(|v| v.parse::<u64>().ok());
                if let (Some(Some(next)), Some(Some(execs))) = (w.next(), w.next()) {
                    out.done = Some((next, execs));
                }
            } else if line.starts_with("FUZZ#") {
                eprintln!("{}: {}", target.name, line.trim_start_matches("FUZZ# "));
            }
        }
    }
    out.status = child.wait().ok();
    out
}

/// Whether a repro child run of `input` ends the way `want` describes (exit code or signal).
fn reproduces(target: &Target, input: &[u8], want: &str, rss_cap: u64) -> bool {
    let dir = artifacts(target.name);
    let file = dir.join(format!(".min-candidate-{}", std::process::id()));
    if std::fs::write(&file, input).is_err() {
        return false;
    }
    let o = spawn(target, &[("FUZZ_REPRO", file.display().to_string())], rss_cap);
    let _ = std::fs::remove_file(&file);
    o.status.is_some_and(|s| describe(s) == want)
}

fn describe(s: ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = s.signal() {
            return format!("signal {sig}");
        }
    }
    format!("exit {}", s.code().unwrap_or(-1))
}

fn supervise(target: &Target, rss_cap: u64) -> Vec<String> {
    let secs = env_u64("FUZZ_SECS", 10);
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut iter = env_u64("FUZZ_FROM", 0);
    let mut execs = 0u64;
    let mut findings: Vec<String> = Vec::new();
    let mut to_minimize: Vec<(PathBuf, String)> = Vec::new();
    let start = Instant::now();
    while Instant::now() < deadline {
        let ms = deadline.saturating_duration_since(Instant::now()).as_millis().to_string();
        let o = spawn(target, &[("FUZZ_FROM", iter.to_string()), ("FUZZ_MS", ms)], rss_cap);
        for f in &o.findings {
            eprintln!("{}: FOUND {f}", target.name);
            findings.push(f.clone());
        }
        let status = o.status.map(describe).unwrap_or_else(|| "not started".into());
        if let Some((next, n)) = o.done {
            execs += n;
            iter = next;
            if status == "exit 0" {
                continue;
            }
        }
        let reached = o.last.unwrap_or(iter);
        execs += reached.saturating_sub(iter);
        match status.as_str() {
            "exit 0" => break,
            "exit 3" | "exit 4" => {
                // the child wrote the input and said which it was: "hang <iter> <path>"
                if let Some(f) = o.findings.last() {
                    let mut w = f.split_whitespace();
                    let _kind = w.next();
                    let at: Option<u64> = w.next().and_then(|v| v.parse().ok());
                    if let Some(p) = w.next() {
                        to_minimize.push((PathBuf::from(p), status.clone()));
                    }
                    iter = at.map_or(reached + 1, |a| a + 1);
                } else {
                    iter = reached + 1;
                }
            }
            "not started" => {
                eprintln!("{}: the child could not be started", target.name);
                break;
            }
            other => {
                // an abort: run the stretch from the last checkpoint again, each input written down first
                let trace = artifacts(target.name).join(format!(".trace-{}", std::process::id()));
                let _ = std::fs::create_dir_all(artifacts(target.name));
                let t = spawn(
                    target,
                    &[
                        ("FUZZ_FROM", reached.to_string()),
                        ("FUZZ_TO", (reached + 65).to_string()),
                        ("FUZZ_MS", "600000".into()),
                        ("FUZZ_TRACE", trace.display().to_string()),
                    ],
                    rss_cap,
                );
                let tstatus = t.status.map(describe).unwrap_or_default();
                if tstatus != "exit 0" && tstatus != "exit 3" && tstatus != "exit 4" {
                    let input = std::fs::read(&trace).unwrap_or_default();
                    let at = t.last.unwrap_or(reached);
                    let note = format!("{tstatus} at input {at} (first seen as {other})\n");
                    let path = write_finding(target.name, "abort", &input, &note);
                    let f = format!("abort {at} {} {tstatus}", path.display());
                    eprintln!("{}: FOUND {f}", target.name);
                    findings.push(f);
                    to_minimize.push((path, tstatus));
                    iter = at + 1;
                } else {
                    eprintln!("{}: {other} near input {reached} did not come again; going on", target.name);
                    iter = reached + 65;
                }
                let _ = std::fs::remove_file(&trace);
            }
        }
    }
    // the first few of each kind: hangs and aborts have no place to tell them apart, and each try is a child run
    let mut per_kind: std::collections::HashMap<String, usize> = Default::default();
    for (path, want) in to_minimize {
        let n = per_kind.entry(want.clone()).or_default();
        *n += 1;
        if *n > 3 {
            continue;
        }
        let Ok(input) = std::fs::read(&path) else { continue };
        let min = minimize(&input, |c| reproduces(target, c, &want, rss_cap), 150, Duration::from_secs(300));
        let _ = std::fs::write(path.with_extension("min.bin"), min);
    }
    let secs = start.elapsed().as_secs_f64().max(1e-9);
    let summary = format!(
        "{}: {execs} inputs in {secs:.0} s ({:.0}/s), next input {iter}, {} findings\n",
        target.name,
        execs as f64 / secs,
        findings.len()
    );
    eprint!("{summary}");
    let dir = artifacts(target.name);
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("stats.txt")) {
        let _ = f.write_all(summary.as_bytes());
    }
    findings
}

// ---------------------------------------------------------------------------------------------------------------
// Inputs of several files

/// What separates the files of one input (a level's level.gltf and level.bin, an archive's header and volume).
pub const PART_SEPARATOR: &[u8] = b"\n\0FUZZ-PART\0\n";

pub fn join_parts(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, p) in parts.iter().enumerate() {
        if k > 0 {
            out.extend_from_slice(PART_SEPARATOR);
        }
        out.extend_from_slice(p);
    }
    out
}

/// The files of an input, at least `n` of them (missing ones empty).
pub fn split_parts(input: &[u8], n: usize) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = input;
    while out.len() + 1 < n {
        match rest.windows(PART_SEPARATOR.len()).position(|w| w == PART_SEPARATOR) {
            Some(at) => {
                out.push(&rest[..at]);
                rest = &rest[at + PART_SEPARATOR.len()..];
            }
            None => break,
        }
    }
    out.push(rest);
    while out.len() < n {
        out.push(&[]);
    }
    out
}
