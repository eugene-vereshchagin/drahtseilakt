// cargo-fuzz target level_txt: the stable fuzzer's entry point run_level_txt (crates/gamedata/tests/fuzz.rs), under libFuzzer.
#![no_main]

#[path = "../../crates/gamedata/tests/fuzz.rs"]
#[allow(dead_code, unused_imports)]
mod tests;

libfuzzer_sys::fuzz_target!(|data: &[u8]| tests::run_level_txt(data));
