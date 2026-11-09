// Fuzz targets of the game CD reader (docs/drahtseilakt/fuzzing.md), on the stable fuzzer in crates/fuzzkit:
//   disc_image  a disc image (plain iso, raw bin, or as given) through its file system: open, find, list, read;
//   cue_sheet   a cue sheet beside a raw bin;
//   iscab       an InstallShield header and one volume: every file listed, the stored ones extracted.
// #[ignore]: `FUZZ_SECS=600 cargo test --release --offline -p cdimage --test fuzz -- --ignored`. Seeds are synthetic
// (made here); no game data. The images are written to a scratch folder under fuzz/artifacts (on disk, not /tmp).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use cdimage::{Image, iscab, iso};
use fuzzkit::{Kind, Target, test_name};

const SECTOR: usize = 2048;
const RAW: usize = 2352;

// --- disc images -------------------------------------------------------------------------------------------------

/// A directory record.
fn rec(name: &[u8], lba: u32, size: u32, dir: bool) -> Vec<u8> {
    let len = 33 + name.len() + (name.len() + 1) % 2;
    let mut r = vec![0u8; len];
    r[0] = len as u8;
    r[2..6].copy_from_slice(&lba.to_le_bytes());
    r[6..10].copy_from_slice(&lba.to_be_bytes());
    r[10..14].copy_from_slice(&size.to_le_bytes());
    r[14..18].copy_from_slice(&size.to_be_bytes());
    r[25] = if dir { 2 } else { 0 };
    r[32] = name.len() as u8;
    r[33..33 + name.len()].copy_from_slice(name);
    r
}

/// A small ISO 9660 image: SETUP/DATA1.HDR and SETUP/DATA2.CAB, a nested folder, a README.
fn iso_image() -> Vec<u8> {
    let mut sectors = vec![vec![0u8; SECTOR]; 24];
    // volume descriptor, then its set terminator
    sectors[16][0] = 1;
    sectors[16][1..6].copy_from_slice(b"CD001");
    sectors[16][6] = 1;
    let root = rec(&[0], 18, SECTOR as u32, true);
    sectors[16][156..156 + root.len()].copy_from_slice(&root);
    sectors[17][0] = 255;
    sectors[17][1..6].copy_from_slice(b"CD001");
    let dir = |entries: &[Vec<u8>]| {
        let mut d = Vec::new();
        entries.iter().for_each(|e| d.extend_from_slice(e));
        d.resize(SECTOR, 0);
        d
    };
    sectors[18] = dir(&[
        rec(&[0], 18, SECTOR as u32, true),
        rec(&[1], 18, SECTOR as u32, true),
        rec(b"SETUP", 19, SECTOR as u32, true),
        rec(b"README.TXT;1", 22, 5, false),
    ]);
    sectors[19] = dir(&[
        rec(&[0], 19, SECTOR as u32, true),
        rec(&[1], 18, SECTOR as u32, true),
        rec(b"DATA1.HDR;1", 20, 600, false),
        rec(b"DATA2.CAB;1", 21, 300, false),
        rec(b"DEEP", 23, SECTOR as u32, true),
    ]);
    sectors[20][..4].copy_from_slice(b"ISc(");
    sectors[21][..5].copy_from_slice(b"plain");
    sectors[22][..5].copy_from_slice(b"hello");
    sectors[23] = dir(&[
        rec(&[0], 23, SECTOR as u32, true),
        rec(&[1], 19, SECTOR as u32, true),
        rec(b"DEEP", 23, SECTOR as u32, true),
    ]);
    sectors.concat()
}

/// A plain image as raw mode 1 sectors (sync, header, data, no error codes).
fn raw(iso: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (n, s) in iso.chunks(SECTOR).enumerate() {
        let mut r = vec![0u8; RAW];
        r[0] = 0;
        r[1..11].fill(0xff);
        r[11] = 0;
        r[12..15].copy_from_slice(&(n as u32).to_be_bytes()[1..]);
        r[15] = 1;
        r[16..16 + s.len()].copy_from_slice(s);
        out.extend(r);
    }
    out
}

/// What the game does with an image: its archive's files by path, every folder listed (bounded), the files read.
fn walk_image(path: &std::path::Path) {
    let Ok(mut img) = Image::open(path) else {
        return;
    };
    for p in [
        "SETUP/DATA1.HDR",
        "SETUP/DATA2.CAB",
        "setup",
        "SETUP/DEEP/DEEP/DEEP/x",
        "",
    ] {
        if let Ok(Some(e)) = iso::find(&mut img, p)
            && !e.dir
        {
            let _ = iso::read(&mut img, &e);
        }
    }
    if let Ok(Some(root)) = iso::find(&mut img, "") {
        let mut stack = vec![(root, 0)];
        let mut seen = 0;
        while let Some((d, depth)) = stack.pop() {
            seen += 1;
            if seen > 64 {
                break;
            }
            let Ok(list) = iso::list(&mut img, &d) else {
                continue;
            };
            for e in list.into_iter().take(64) {
                if e.dir && depth < 4 {
                    stack.push((e, depth + 1));
                } else if !e.dir {
                    let _ = iso::read(&mut img, &e);
                }
            }
        }
    }
}

/// The first byte says how the rest is written: a plain iso (padded to whole sectors), raw sectors, or as it is.
pub fn run_disc_image(input: &[u8]) {
    let Some((&how, bytes)) = input.split_first() else {
        return;
    };
    let mut data = bytes.to_vec();
    match how % 3 {
        0 => data.resize(data.len().div_ceil(SECTOR) * SECTOR, 0),
        1 => {
            data.resize(data.len().div_ceil(SECTOR) * SECTOR, 0);
            data = raw(&data);
        }
        _ => {}
    }
    let path = fuzzkit::scratch().join("disc.iso");
    if std::fs::write(&path, &data).is_ok() {
        walk_image(&path);
    }
}

fn disc_seeds() -> Vec<Vec<u8>> {
    let iso = iso_image();
    (0u8..3).map(|k| [vec![k], iso.clone()].concat()).collect()
}

#[test]
#[ignore]
fn fuzz_disc_image() {
    let mut t = Target::new("disc_image", test_name!("fuzz_disc_image"), Kind::Binary);
    t.max_len = 1 << 20;
    fuzzkit::fuzz(t, disc_seeds, run_disc_image);
}

/// A cue sheet (first part) and the raw image beside it (second part, padded to whole raw sectors).
pub fn run_cue_sheet(input: &[u8]) {
    let parts = fuzzkit::split_parts(input, 2);
    let dir = fuzzkit::scratch().join("cue");
    let _ = std::fs::remove_dir_all(&dir);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let mut bin = parts[1].to_vec();
    bin.resize(bin.len().div_ceil(RAW) * RAW, 0);
    let cue = dir.join("disc.cue");
    if std::fs::write(&cue, parts[0]).is_ok() && std::fs::write(dir.join("disc.bin"), &bin).is_ok()
    {
        walk_image(&cue);
    }
}

fn cue_seeds() -> Vec<Vec<u8>> {
    let bin = raw(&iso_image());
    [
        "FILE \"disc.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n",
        "\u{feff}REM x\r\nfile disc.bin binary\r\n",
        "FILE \"../disc.bin\" BINARY\n",
    ]
    .iter()
    .map(|c| fuzzkit::join_parts(&[c.as_bytes(), &bin]))
    .collect()
}

#[test]
#[ignore]
fn fuzz_cue_sheet() {
    let mut t = Target::new("cue_sheet", test_name!("fuzz_cue_sheet"), Kind::Text);
    t.max_len = 1 << 20;
    t.dict = &[
        "FILE",
        "BINARY",
        "TRACK",
        "MODE1/2352",
        "MODE2/2352",
        "INDEX",
        "\"",
        "disc.bin",
        "..",
        "/",
    ];
    fuzzkit::fuzz(t, cue_seeds, run_cue_sheet);
}

// --- InstallShield archives --------------------------------------------------------------------------------------

const DESCRIPTOR: usize = 0x57;

/// A header with one directory and the files given (name, flags, expanded, compressed, offset), DATA1.HDR's layout.
fn header(files: &[(&str, u16, u64, u64, u64)]) -> Vec<u8> {
    let desc = 0x200usize;
    let table_off = 0x100u32;
    let table = desc + table_off as usize;
    let mut h = vec![0u8; table + 0x40 + files.len() * (DESCRIPTOR + 32)];
    h[..4].copy_from_slice(b"ISc(");
    h[4..8].copy_from_slice(&0x0100_600cu32.to_le_bytes());
    h[0x0c..0x10].copy_from_slice(&(desc as u32).to_le_bytes());
    let put32 =
        |h: &mut Vec<u8>, at: usize, v: u32| h[at..at + 4].copy_from_slice(&v.to_le_bytes());
    put32(&mut h, desc + 0x0c, table_off);
    put32(&mut h, desc + 0x1c, 1);
    put32(&mut h, desc + 0x28, files.len() as u32);
    let names = 0x40usize;
    put32(&mut h, desc + 0x2c, names as u32 + files.len() as u32 * 32);
    put32(&mut h, table, 8);
    h[table + 8..table + 8 + 18].copy_from_slice(b"3D Entities\\Level\0");
    for (i, (name, flags, exp, comp, off)) in files.iter().enumerate() {
        let n = table + names + i * 32;
        h[n..n + name.len()].copy_from_slice(name.as_bytes());
        let p = table + names + files.len() * 32 + i * DESCRIPTOR;
        h[p..p + 2].copy_from_slice(&flags.to_le_bytes());
        h[p + 2..p + 10].copy_from_slice(&exp.to_le_bytes());
        h[p + 10..p + 18].copy_from_slice(&comp.to_le_bytes());
        h[p + 18..p + 26].copy_from_slice(&off.to_le_bytes());
        put32(&mut h, p + 0x3a, (names + i * 32) as u32);
        h[p + 0x55..p + 0x57].copy_from_slice(&2u16.to_le_bytes());
    }
    h
}

/// Raw deflate chunks, each led by its length, as the archive keeps a compressed file.
fn chunks(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in data.chunks(1000) {
        let z = miniz_oxide::deflate::compress_to_vec(part, 6);
        out.extend_from_slice(&(z.len() as u16).to_le_bytes());
        out.extend_from_slice(&z);
    }
    out
}

pub fn run_iscab(input: &[u8]) {
    let parts = fuzzkit::split_parts(input, 2);
    let Ok(cab) = iscab::parse(parts[0]) else {
        return;
    };
    for f in cab.files.iter().filter(|f| f.stored()).take(64) {
        let _ = f.parts();
        if let Ok(bytes) = iscab::extract(f, parts[1]) {
            assert_eq!(
                bytes.len() as u64,
                f.expanded,
                "an extracted file of another size than its own"
            );
        }
    }
}

fn iscab_seeds() -> Vec<Vec<u8>> {
    let data: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
    let packed = chunks(&data);
    let mut volume = vec![0u8; 0x200];
    volume.extend_from_slice(&packed);
    volume.extend_from_slice(b"plain");
    let plain_at = 0x200 + packed.len() as u64;
    let h = header(&[
        ("Level_01.NMO", 4, 5000, packed.len() as u64, 0x200),
        ("readme.txt", 0, 5, 5, plain_at),
        ("", 8, 0, 0, 0),
        ("split.bin", 1, 5, 5, plain_at),
    ]);
    vec![
        fuzzkit::join_parts(&[&h, &volume]),
        fuzzkit::join_parts(&[&header(&[("a", 0, 1, 1, 1)]), b"xy"]),
    ]
}

#[test]
#[ignore]
fn fuzz_iscab() {
    let mut t = Target::new("iscab", test_name!("fuzz_iscab"), Kind::Binary);
    t.max_len = 1 << 20;
    fuzzkit::fuzz(t, iscab_seeds, run_iscab);
}

// --- committed repros --------------------------------------------------------------------------------------------

#[test]
fn fuzz_regressions() {
    fuzzkit::regressions("disc_image", run_disc_image);
    fuzzkit::regressions("cue_sheet", run_cue_sheet);
    fuzzkit::regressions("iscab", run_iscab);
}

/// The synthetic seeds are what they mean to be (a seed the reader refuses would fuzz only its first checks).
#[test]
fn fuzz_seeds_are_read() {
    for (k, image) in [iso_image(), raw(&iso_image())].iter().enumerate() {
        let path = fuzzkit::scratch().join(format!("seed{k}.iso"));
        std::fs::write(&path, image).unwrap();
        let mut img = Image::open(&path).unwrap();
        let e = iso::find(&mut img, "SETUP/DATA1.HDR").unwrap().unwrap();
        assert_eq!(&iso::read(&mut img, &e).unwrap()[..4], b"ISc(");
    }
    let parts = iscab_seeds();
    let p = fuzzkit::split_parts(&parts[0], 2);
    let cab = iscab::parse(p[0]).unwrap();
    assert_eq!(iscab::extract(&cab.files[0], p[1]).unwrap().len(), 5000);
}
