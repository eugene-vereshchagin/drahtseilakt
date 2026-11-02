// cargo-fuzz target iscab: the stable fuzzer's entry point run_iscab (crates/cdimage/tests/fuzz.rs), under libFuzzer.
#![no_main]

#[path = "../../crates/cdimage/tests/fuzz.rs"]
#[allow(dead_code, unused_imports)]
mod tests;

libfuzzer_sys::fuzz_target!(|data: &[u8]| tests::run_iscab(data));
