// Container tests on synthetic files the test builds itself (no game data needed), plus a check that no
// damaged variant of a valid file can make the reader panic.

// The hostile-input lints guard library code; a test that panics is a failing test, which is what we want.
#![allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::unwrap_used
)]

use nmo::{Error, Part};

const PLAIN: u32 = 0;
const COMPRESSED: u32 = 8;

struct Spec {
    header1: Vec<u8>,
    data: Vec<u8>,
    compress: bool,
    included: Vec<(Vec<u8>, Vec<u8>)>,
}

fn adler_virtools(chunks: &[&[u8]]) -> u32 {
    let (mut a, mut b) = (0u64, 0u64);
    for chunk in chunks {
        for &byte in *chunk {
            a = (a + u64::from(byte)) % 65_521;
            b = (b + a) % 65_521;
        }
    }
    ((b << 16) | a) as u32
}

fn build(spec: &Spec) -> Vec<u8> {
    let pack = |d: &[u8]| {
        if spec.compress {
            miniz_oxide::deflate::compress_to_vec_zlib(d, 6)
        } else {
            d.to_vec()
        }
    };
    let (h1, data) = (pack(&spec.header1), pack(&spec.data));
    let fields: [u32; 14] = [
        0,           // checksum, filled in below
        0x1302_2002, // ck version
        8,           // file version
        0,           // unused
        if spec.compress { COMPRESSED } else { PLAIN },
        h1.len() as u32,
        data.len() as u32,
        spec.data.len() as u32,
        1,           // managers
        2,           // objects
        3,           // max id
        0,           // product version
        0x0201_0001, // product build
        spec.header1.len() as u32,
    ];
    let mut head: Vec<u8> = b"Nemo Fi\0".to_vec();
    fields
        .iter()
        .for_each(|f| head.extend_from_slice(&f.to_le_bytes()));
    let sum = adler_virtools(&[&head, &h1, &data]);
    head[8..12].copy_from_slice(&sum.to_le_bytes());
    let mut out = head;
    out.extend_from_slice(&h1);
    out.extend_from_slice(&data);
    for (name, bytes) in &spec.included {
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

fn sample(compress: bool) -> Spec {
    Spec {
        header1: (0..300u32).flat_map(|i| i.to_le_bytes()).collect(),
        data: (0..2000u32).map(|i| (i % 7) as u8).collect(),
        compress,
        included: vec![(b"vtthumbnail.jpg".to_vec(), vec![0xFF, 0xD8, 0xFF, 0xE0])],
    }
}

#[test]
fn plain_and_compressed_round_trip() -> Result<(), Error> {
    for compress in [false, true] {
        let spec = sample(compress);
        let file = nmo::read(&build(&spec))?;
        assert_eq!(file.header1, spec.header1);
        assert_eq!(file.data, spec.data);
        assert!(file.checksum_matches());
        assert_eq!(file.included.len(), 1);
        assert_eq!(file.included[0].name, b"vtthumbnail.jpg");
        assert_eq!(file.header.object_count, 2);
    }
    Ok(())
}

#[test]
fn checksum_mismatch_is_reported_not_refused() -> Result<(), Error> {
    let mut bytes = build(&sample(false));
    let last = bytes.len() - 30; // inside the data, before the included file
    bytes[last] ^= 0x55;
    let file = nmo::read(&bytes)?;
    assert!(!file.checksum_matches());
    Ok(())
}

#[test]
fn rejects_what_is_not_virtools() {
    assert_eq!(nmo::read(b"").err(), Some(Error::NotVirtools));
    assert_eq!(
        nmo::read(b"PK\x03\x04 not a level at all").err(),
        Some(Error::NotVirtools)
    );
}

#[test]
fn rejects_unknown_versions_and_huge_sizes() {
    let mut bytes = build(&sample(false));
    bytes[16..20].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        nmo::read(&bytes).err(),
        Some(Error::UnsupportedVersion { file_version: 2 })
    );

    let mut bytes = build(&sample(true));
    bytes[36..40].copy_from_slice(&u32::MAX.to_le_bytes()); // data unpacked size
    assert!(matches!(
        nmo::read(&bytes),
        Err(Error::TooLarge {
            part: Part::Data,
            ..
        })
    ));
}

#[test]
fn rejects_corrupt_compression() {
    let mut bytes = build(&sample(true));
    let h1_packed = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as usize;
    let data_packed = u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]) as usize;
    let start = 64 + h1_packed;
    for b in &mut bytes[start + 2..start + data_packed - 4] {
        *b = 0xAA;
    }
    assert_eq!(
        nmo::read(&bytes).err(),
        Some(Error::Decompress { part: Part::Data })
    );
}

#[test]
fn every_truncation_fails_cleanly() {
    // Cutting a valid file anywhere must give an error (or, inside the included files, a shorter list),
    // never a panic.
    for compress in [false, true] {
        let bytes = build(&sample(compress));
        for len in 0..bytes.len() {
            let _ = nmo::read(&bytes[..len]);
        }
    }
}

#[test]
fn every_single_byte_corruption_fails_cleanly() {
    let bytes = build(&sample(true));
    for i in 0..bytes.len() {
        for x in [0x00u8, 0xFF, 0x80] {
            let mut damaged = bytes.clone();
            damaged[i] = x;
            let _ = nmo::read(&damaged);
        }
    }
}
