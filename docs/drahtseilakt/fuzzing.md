# Fuzzing the parsers

Every reader of a file the game did not write itself — `.nmo`/`.cmo` levels and community maps, the open level
format, pictures, sounds, the CD image and its InstallShield archive, replays, hint files, level folders' `level.txt`,
sound themes, the settings and highscore files — has a fuzz target. There are two ways to run them.

## The stable fuzzer (runs anywhere, offline)

`crates/fuzzkit` is a small mutation fuzzer with no dependencies. Each target is an `#[ignore]` test next to the
parser's other tests; it mutates a corpus of seeds (byte flips, interesting values, inserts, deletes, copies, splices,
truncations, length fields nudged, and for text formats lines, words and numbers) with a fixed xorshift generator, so
every input can be made again from its number. Each input runs under a guard: a panic is caught, an input that runs
past the deadline or takes more memory than the target's cap ends the run, and an abort (an allocation failure under
the address-space limit, a stack overflow) is found again by replaying the last stretch.

```sh
scripts/fuzz.sh                    # every target, 600 s each, 4 at once
scripts/fuzz.sh 3600 2 nmo_file    # one target, an hour
FUZZ_SECS=60 cargo test --release --offline -p level --test fuzz fuzz_nmo_file -- --ignored --nocapture
```

Targets: `nmo_file`, `nmo_chunk`, `open_level` (crate `level`); `image`, `level_txt`, `theme_txt`, `sound_table`
(`gamedata`); `disc_image`, `cue_sheet`, `iscab` (`cdimage`); `replay`, `hints`, `wav`, `ogg`, `highscore`,
`settings` (the game, `crates/game/src/fuzz_tests.rs`).

Seeds are small synthetic files the tests make, the game's own shipped files, and — when present — the player's
files (`OPENBALLANCE_DATA`, `OPENBALLANCE_MAPS`, the profile's original and community levels), read at run time and
never written into the repository. `FUZZ_NO_REAL=1` leaves them out.

Settings: `FUZZ_SECS` (time per target), `FUZZ_SEED` (which inputs; `scripts/fuzz.sh` takes a new one each run),
`FUZZ_TIMEOUT_MS` (one input's deadline, 10 s), `FUZZ_RSS_MB` (memory cap), `FUZZ_REPRO=<file>` (run one input and
say what it does).

What is found goes to `fuzz/artifacts/<target>/` (ignored by git): `crash-*`, `hang-*`, `oom-*` or `abort-*`, each
`.bin` with a `.txt` saying what happened and a minimised `.min.bin`. A fix gets a regression test beside the parser,
and a synthetic repro (never game data) in `fuzz/regressions/<target>/`, which the ordinary tests replay.

## cargo-fuzz (libFuzzer, nightly)

`fuzz/` is a cargo-fuzz crate outside the workspace (the game's build never needs it). Its targets run the same
functions as the stable fuzzer's tests:

```sh
FUZZ_DUMP_CORPUS=1 cargo test --release --offline -p level --test fuzz fuzz_nmo_file -- --ignored  # seeds -> fuzz/corpus/
cd fuzz && cargo +nightly fuzz run nmo_file corpus/nmo_file
```

The game is a binary crate, so of its own parsers only `replay` (which stands alone) has a cargo-fuzz target; the
others (`hints`, `wav`, `ogg`, `highscore`, `settings`) run on the stable fuzzer.
