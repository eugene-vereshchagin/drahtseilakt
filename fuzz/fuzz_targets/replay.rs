// cargo-fuzz target replay: a replay file read and written back (crates/game/src/replay.rs, which stands alone and is
// included here; the game is a binary crate, so its other parsers are fuzzed by the stable fuzzer only).
#![no_main]

#[path = "../../crates/game/src/replay.rs"]
#[allow(dead_code)]
mod replay;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    if let Ok(r) = replay::Replay::parse(text) {
        assert!(replay::Replay::parse(&r.to_text()).is_ok());
    }
});
