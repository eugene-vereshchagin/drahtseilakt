// The fuzzer's own check: targets with planted bugs (a panic, a hang, a huge allocation, a stack overflow) must each
// be found, written to fuzz/artifacts/selftest_*/ and minimised. #[ignore]d (each takes seconds and finds things on
// purpose): `FUZZ_SECS=5 cargo test --release --offline -p fuzzkit --test selftest -- --ignored --test-threads=1`;
// every test here is expected to fail with "new findings".

#![allow(clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects)]

use fuzzkit::{Kind, Target, test_name};

fn seeds() -> Vec<Vec<u8>> {
    vec![b"hello world 12 34".to_vec()]
}

fn planted(input: &[u8]) -> bool {
    input.windows(2).any(|w| w == b"-1")
}

#[test]
#[ignore]
fn selftest_panic() {
    fuzzkit::fuzz(Target::new("selftest_panic", test_name!("selftest_panic"), Kind::Text), seeds, |b| {
        if planted(b) {
            panic!("planted");
        }
    });
}

#[test]
#[ignore]
fn selftest_hang() {
    fuzzkit::fuzz(Target::new("selftest_hang", test_name!("selftest_hang"), Kind::Text), seeds, |b| {
        if planted(b) {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    });
}

#[test]
#[ignore]
fn selftest_oom() {
    let mut t = Target::new("selftest_oom", test_name!("selftest_oom"), Kind::Text);
    t.rss_mb = 128;
    fuzzkit::fuzz(t, seeds, |b| {
        if planted(b) {
            let v = std::hint::black_box(vec![1u8; 400 << 20]);
            std::thread::sleep(std::time::Duration::from_millis(500));
            std::hint::black_box(&v);
        }
    });
}

fn deep(n: u64) -> u64 {
    let pad = [n; 64];
    if n == 0 { 0 } else { std::hint::black_box(deep(std::hint::black_box(n - 1))).wrapping_add(std::hint::black_box(&pad)[(n % 64) as usize]) }
}

#[test]
#[ignore]
fn selftest_abort() {
    fuzzkit::fuzz(Target::new("selftest_abort", test_name!("selftest_abort"), Kind::Text), seeds, |b| {
        if planted(b) {
            std::hint::black_box(deep(std::hint::black_box(u64::MAX)));
        }
    });
}
