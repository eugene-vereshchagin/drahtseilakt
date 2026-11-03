// cargo-fuzz target open_level: the stable fuzzer's entry point run_open_level (crates/level/tests/fuzz.rs), under libFuzzer.
#![no_main]

#[path = "../../crates/level/tests/fuzz.rs"]
#[allow(dead_code, unused_imports)]
mod tests;

libfuzzer_sys::fuzz_target!(|data: &[u8]| tests::run_open_level(data));
