// cargo-fuzz target cue_sheet: the stable fuzzer's entry point run_cue_sheet (crates/cdimage/tests/fuzz.rs), under libFuzzer.
#![no_main]

#[path = "../../crates/cdimage/tests/fuzz.rs"]
#[allow(dead_code, unused_imports)]
mod tests;

libfuzzer_sys::fuzz_target!(|data: &[u8]| tests::run_cue_sheet(data));
