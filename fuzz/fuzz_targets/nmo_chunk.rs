// cargo-fuzz target nmo_chunk: the stable fuzzer's entry point run_nmo_chunk (crates/level/tests/fuzz.rs), under libFuzzer.
#![no_main]

#[path = "../../crates/level/tests/fuzz.rs"]
#[allow(dead_code, unused_imports)]
mod tests;

libfuzzer_sys::fuzz_target!(|data: &[u8]| tests::run_nmo_chunk(data));
