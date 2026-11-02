// The player's game CD (OPENBALLANCE_CD: its .cue, .bin or .iso; skipped without it): every file of its
// InstallShield archive is extracted whole, and damaged copies of the archive's header and of a volume are read
// without a panic.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use cdimage::{Image, iscab, iso};

fn volume(img: &mut Image, n: u16) -> Vec<u8> {
    let e = iso::find(img, &format!("SETUP/DATA{n}.CAB"))
        .unwrap()
        .unwrap();
    iso::read(img, &e).unwrap()
}

#[test]
fn the_game_cd_extracts_whole() {
    let Ok(path) = std::env::var("OPENBALLANCE_CD") else {
        eprintln!("OPENBALLANCE_CD not set; skipped");
        return;
    };
    let mut img = Image::open(path.as_ref()).unwrap();
    let hdr = iso::find(&mut img, "SETUP/DATA1.HDR")
        .unwrap()
        .expect("SETUP/DATA1.HDR");
    let hdr = iso::read(&mut img, &hdr).unwrap();
    let cab = iscab::parse(&hdr).unwrap();
    let mut volumes = std::collections::HashMap::new();
    let mut extracted = 0;
    for f in cab.files.iter().filter(|f| f.stored()) {
        let v = volumes
            .entry(f.volume)
            .or_insert_with(|| volume(&mut img, f.volume));
        let data = iscab::extract(f, v).unwrap_or_else(|e| panic!("{}: {e}", f.name));
        assert_eq!(data.len() as u64, f.expanded);
        extracted += 1;
    }
    eprintln!("{extracted} files extracted");
    assert!(
        cab.files
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case("base.cmo"))
    );

    // damage: flipped bytes and edge words in the header, then in the first file's data
    let mut seed = 0x5eed_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..2000 {
        let mut h = hdr.clone();
        for _ in 0..1 + next() % 6 {
            let i = (next() % h.len() as u64) as usize;
            h[i] = if next() % 2 == 0 {
                h[i] ^ (1 << (next() % 8))
            } else {
                0xff
            };
        }
        if let Ok(c) = iscab::parse(&h) {
            for f in c.files.iter().take(40) {
                if let Some(v) = volumes.get(&f.volume) {
                    let _ = iscab::extract(f, v);
                }
                let _ = f.parts();
            }
        }
    }
    let first = cab
        .files
        .iter()
        .find(|f| f.stored() && f.volume == 2)
        .unwrap();
    let v = &volumes[&2];
    let span = first.offset as usize..(first.offset + first.compressed) as usize;
    for _ in 0..300 {
        let mut d = v[span.clone()].to_vec();
        for _ in 0..1 + next() % 4 {
            let i = (next() % d.len() as u64) as usize;
            d[i] ^= 1 << (next() % 8);
        }
        let mut vol = vec![0u8; first.offset as usize];
        vol.extend_from_slice(&d);
        let _ = iscab::extract(first, &vol);
    }
}
