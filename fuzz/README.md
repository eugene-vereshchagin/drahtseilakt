# fuzz/

- `fuzz_targets/` — cargo-fuzz (libFuzzer) targets for nightly: `cargo +nightly fuzz run <target>` from this folder.
  Not part of the workspace and never built by default; its own `Cargo.lock` is made the first time it is built with
  the network (libfuzzer-sys was not in the offline cache when the targets were written, so they have not been built
  yet). Each target calls the same `run_*` function as the stable fuzzer's test of that name.
- `regressions/<target>/` — minimised, synthetic repros of bugs fuzzing found (never game data); the ordinary tests
  (`fuzz_regressions` in each crate) replay them.
- `artifacts/`, `corpus/` — made by runs, ignored by git (they may hold the player's files).

How to run, the targets and the settings: [docs/drahtseilakt/fuzzing.md](../docs/drahtseilakt/fuzzing.md). The stable
fuzzer (`crates/fuzzkit`, `scripts/fuzz.sh`) runs anywhere, offline, without nightly.
