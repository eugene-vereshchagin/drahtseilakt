// The picture patcher (nmo::patch) on synthetic files the test builds itself: the new pixels read back, nothing
// else in the file changes, every refusal leaves no half-written file, and no damaged file makes it panic. With
// the shipped community maps present (assets/levels/community, not in git), patching every saved picture with
// itself gives back the same data part.

#![allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::unwrap_used
)]

use nmo::patch::{NewPicture, replace_pictures};

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

/// A state chunk of linked sections, saved with file references.
fn chunk(sections: &[(u32, Vec<u32>)]) -> Vec<u8> {
    let mut words = Vec::new();
    for (k, (id, data)) in sections.iter().enumerate() {
        let next = words.len() + 2 + data.len();
        words.push(*id);
        words.push(if k + 1 == sections.len() {
            0
        } else {
            next as u32
        });
        words.extend(data);
    }
    let version: u32 = 7 | (31 << 8) | (7 << 16) | (8 << 24);
    let mut out = version.to_le_bytes().to_vec();
    out.extend_from_slice(&(words.len() as u32).to_le_bytes());
    words
        .iter()
        .for_each(|w| out.extend_from_slice(&w.to_le_bytes()));
    out
}

/// A saved picture's words: bpp, sides, masks, compression, then the planes (blue, green, red, alpha), rows bottom
/// up, from RGBA rows top down.
fn picture_words(w: u32, h: u32, rgba: &[u8], alpha: bool) -> Vec<u32> {
    let mut out = vec![32, w, h, 0xFF00_0000, 0xFF_0000, 0xFF00, 0xFF, 0];
    let channels: &[usize] = if alpha { &[2, 1, 0, 3] } else { &[2, 1, 0] };
    for &c in channels {
        let mut plane = Vec::new();
        for row in (0..h as usize).rev() {
            for col in 0..w as usize {
                plane.push(rgba[(row * w as usize + col) * 4 + c]);
            }
        }
        out.push(plane.len() as u32);
        while plane.len() % 4 != 0 {
            plane.push(0xEE); // padding the patcher must leave alone
        }
        out.extend(
            plane
                .chunks(4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        );
    }
    out
}

fn pixels(w: u32, h: u32, seed: u8, opaque: bool) -> Vec<u8> {
    (0..w * h * 4)
        .map(|i| {
            if opaque && i % 4 == 3 {
                0xFF
            } else {
                (i as u8).wrapping_mul(31).wrapping_add(seed)
            }
        })
        .collect()
}

/// How a sample file is saved.
#[derive(Clone, Copy)]
struct Saved {
    version: u32,
    write_mode: u32,
    pack_header1: bool,
    pack_data: bool,
}

impl Saved {
    fn plain() -> Saved {
        Saved {
            version: 8,
            write_mode: 0,
            pack_header1: false,
            pack_data: false,
        }
    }
    fn packed() -> Saved {
        Saved {
            write_mode: 8,
            pack_header1: true,
            pack_data: true,
            ..Saved::plain()
        }
    }
}

/// A file with a manager, a material, a texture of two saved pictures (3x2 with alpha, 2x3 without), a texture
/// with only a file name; stored packed or plain, with an included file.
fn sample(compress: bool) -> Vec<u8> {
    sample_saved(if compress {
        Saved::packed()
    } else {
        Saved::plain()
    })
}

fn sample_saved(saved: Saved) -> Vec<u8> {
    let objects: Vec<(u32, &str, Vec<u8>)> = vec![
        (30, "Mat", chunk(&[(0x1000, vec![0; 12])])),
        (
            31,
            "Pic",
            chunk(&[
                (0x0001_0000, vec![2, 0, 0]),
                (0x0002_0000, {
                    let mut w = vec![2];
                    w.extend(picture_words(3, 2, &pixels(3, 2, 1, false), true));
                    w.extend(picture_words(2, 3, &pixels(2, 3, 2, true), false));
                    w
                }),
                (0x002F_F000, vec![0]),
            ]),
        ),
        (31, "Named", chunk(&[(0x0001_0000, vec![1, 0])])),
    ];
    let (mut h1, mut data) = (Vec::new(), Vec::new());
    data.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    data.extend_from_slice(&4u32.to_le_bytes());
    data.extend_from_slice(&7u32.to_le_bytes());
    for (k, (class, name, raw)) in objects.iter().enumerate() {
        h1.extend_from_slice(&(k as u32 + 1).to_le_bytes());
        h1.extend_from_slice(&class.to_le_bytes());
        h1.extend_from_slice(&0u32.to_le_bytes());
        h1.extend_from_slice(&(name.len() as u32).to_le_bytes());
        h1.extend_from_slice(name.as_bytes());
        data.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        data.extend_from_slice(raw);
    }
    let pack = |d: &[u8], compress: bool| {
        if compress {
            miniz_oxide::deflate::compress_to_vec_zlib(d, 1)
        } else {
            d.to_vec()
        }
    };
    let (p1, pd) = (pack(&h1, saved.pack_header1), pack(&data, saved.pack_data));
    let fields: [u32; 14] = [
        0,
        0x1302_2002,
        saved.version,
        0,
        saved.write_mode,
        p1.len() as u32,
        pd.len() as u32,
        data.len() as u32,
        1,
        objects.len() as u32,
        objects.len() as u32 + 1,
        0,
        0x0201_0001,
        h1.len() as u32,
    ];
    let mut head = b"Nemo Fi\0".to_vec();
    fields
        .iter()
        .for_each(|f| head.extend_from_slice(&f.to_le_bytes()));
    let sum = adler_virtools(&[&head, &p1, &pd]);
    head[8..12].copy_from_slice(&sum.to_le_bytes());
    let mut out = head;
    out.extend_from_slice(&p1);
    out.extend_from_slice(&pd);
    out.extend_from_slice(&15u32.to_le_bytes());
    out.extend_from_slice(b"vtthumbnail.jpg");
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xD8, 0xFF]);
    out
}

/// The saved pictures of object `i`: (width, height, rgba) per slot.
fn pictures(bytes: &[u8], i: usize) -> Vec<(u32, u32, Vec<u8>)> {
    let f = nmo::read(bytes).unwrap();
    let doc = nmo::Document::parse(&f).unwrap();
    let t = nmo::scene::texture(doc.objects[i].chunk.as_ref().unwrap()).unwrap();
    std::iter::once(t.embedded)
        .chain(t.slots.into_iter().map(|s| s.embedded))
        .flatten()
        .collect()
}

#[test]
fn new_pictures_read_back_and_nothing_else_changes() {
    for compress in [false, true] {
        let before = sample(compress);
        assert_eq!(pictures(&before, 1).len(), 2, "the sample's own pictures");
        let (a, b) = (pixels(3, 2, 90, false), pixels(2, 3, 50, true));
        let after = replace_pictures(
            &before,
            &[
                NewPicture {
                    object: 1,
                    slot: 0,
                    width: 3,
                    height: 2,
                    rgba: &a,
                },
                NewPicture {
                    object: 1,
                    slot: 1,
                    width: 2,
                    height: 3,
                    rgba: &b,
                },
            ],
        )
        .unwrap();
        let got = pictures(&after, 1);
        assert_eq!(got[0], (3, 2, a.clone()));
        assert_eq!(got[1], (2, 3, b.clone()));
        let (fb, fa) = (nmo::read(&before).unwrap(), nmo::read(&after).unwrap());
        assert!(fa.checksum_matches());
        assert_eq!(fa.header1, fb.header1);
        assert_eq!(fa.included, fb.included);
        assert_eq!(fa.data.len(), fb.data.len());
        // only the plane bytes changed: 3x2x4 and 2x3x3 bytes at most, never the padding or another object
        let changed = fa.data.iter().zip(&fb.data).filter(|(x, y)| x != y).count();
        assert!(changed <= 6 * 4 + 6 * 3, "{changed} bytes changed");
        assert!(fa.data.windows(2).filter(|w| w == &[0xEE, 0xEE]).count() >= 1);
        let header_changes: Vec<usize> = (0..64).filter(|&k| before[k] != after[k]).collect();
        assert!(
            header_changes
                .iter()
                .all(|&k| (8..12).contains(&k) || (32..36).contains(&k)),
            "only the checksum and the data's packed size change: {header_changes:?}"
        );
        // the same pictures again give the same file
        let again = replace_pictures(
            &after,
            &[NewPicture {
                object: 1,
                slot: 0,
                width: 3,
                height: 2,
                rgba: &a,
            }],
        )
        .unwrap();
        assert_eq!(nmo::read(&again).unwrap().data, fa.data);
        if !compress {
            assert_eq!(before.len(), after.len(), "a plain file keeps its length");
        }
    }
}

#[test]
fn what_cannot_be_patched_exactly_is_refused() {
    let file = sample(true);
    let ok = pixels(3, 2, 9, false);
    let opaque = pixels(2, 3, 9, true);
    let transparent = pixels(2, 3, 9, false);
    let short = vec![0u8; 3 * 2 * 4 - 1];
    let cases: Vec<(usize, usize, (u32, u32), &[u8], &str)> = vec![
        (0, 0, (3, 2), &ok, "a material"),
        (2, 0, (3, 2), &ok, "a texture with no saved picture"),
        (1, 2, (3, 2), &ok, "a slot past the saved ones"),
        (9, 0, (3, 2), &ok, "an object past the table"),
        (1, 0, (3, 2), &short, "too few pixels"),
        (1, 0, (2, 3), &opaque, "another picture's sides"),
        (
            1,
            1,
            (2, 3),
            &transparent,
            "transparency where there is no alpha",
        ),
    ];
    for (object, slot, (width, height), rgba, what) in cases {
        let r = replace_pictures(
            &file,
            &[NewPicture {
                object,
                slot,
                width,
                height,
                rgba,
            }],
        );
        assert!(r.is_err(), "{what}: {r:?}");
    }
    // two different pictures for one slot cannot both read back
    let other = pixels(3, 2, 10, false);
    assert!(
        replace_pictures(
            &file,
            &[
                NewPicture {
                    object: 1,
                    slot: 0,
                    width: 3,
                    height: 2,
                    rgba: &ok
                },
                NewPicture {
                    object: 1,
                    slot: 0,
                    width: 3,
                    height: 2,
                    rgba: &other
                },
            ],
        )
        .is_err()
    );
    assert!(replace_pictures(b"not a file", &[]).is_err());
}

#[test]
fn damaged_files_never_panic() {
    let rgba = pixels(3, 2, 77, false);
    let pics = [NewPicture {
        object: 1,
        slot: 0,
        width: 3,
        height: 2,
        rgba: &rgba,
    }];
    for compress in [false, true] {
        let file = sample(compress);
        for cut in 0..file.len() {
            let _ = replace_pictures(&file[..cut], &pics);
        }
        for k in 0..file.len() {
            for flip in [0x01u8, 0x80, 0xFF] {
                let mut bad = file.clone();
                bad[k] ^= flip;
                if let Ok(out) = replace_pictures(&bad, &pics) {
                    // whatever it accepted holds the picture asked for
                    assert_eq!(pictures(&out, 1)[0].2, rgba, "byte {k} ^ {flip:#x}");
                }
            }
        }
    }
}

#[test]
fn shipped_maps_patched_with_their_own_pictures_stay_the_same() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/levels/community");
    let Ok(levels) = std::fs::read_dir(&dir) else {
        eprintln!("no shipped community maps; skipped");
        return;
    };
    let mut maps = 0;
    for level in levels.flatten() {
        let Ok(files) = std::fs::read_dir(level.path()) else {
            continue;
        };
        for f in files.flatten() {
            let path = f.path();
            if !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("nmo"))
            {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let file = nmo::read(&bytes).unwrap();
            let doc = nmo::Document::parse(&file).unwrap();
            let mut own: Vec<(usize, usize, u32, u32, Vec<u8>)> = Vec::new();
            for (i, o) in doc.of_class(nmo::class::TEXTURE) {
                let Some(c) = &o.chunk else { continue };
                let Ok(t) = nmo::scene::texture(c) else {
                    continue;
                };
                let slots =
                    std::iter::once(t.embedded).chain(t.slots.into_iter().map(|s| s.embedded));
                for (s, p) in slots.enumerate() {
                    if let Some((w, h, rgba)) = p {
                        own.push((i, s, w, h, rgba));
                    }
                }
            }
            let pics: Vec<NewPicture<'_>> = own
                .iter()
                .map(|(object, slot, width, height, rgba)| NewPicture {
                    object: *object,
                    slot: *slot,
                    width: *width,
                    height: *height,
                    rgba,
                })
                .collect();
            let out = replace_pictures(&bytes, &pics).unwrap();
            assert_eq!(
                nmo::read(&out).unwrap().data,
                file.data,
                "{}",
                path.display()
            );
            maps += 1;
        }
    }
    eprintln!("{maps} shipped maps patched with their own pictures");
}

fn one_picture(rgba: &[u8]) -> [NewPicture<'_>; 1] {
    [NewPicture {
        object: 1,
        slot: 0,
        width: 3,
        height: 2,
        rgba,
    }]
}

fn header_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[test]
fn only_version_8_and_9_files_saved_whole_are_patched() {
    let rgba = pixels(3, 2, 33, false);
    for version in [5, 6, 7] {
        let file = sample_saved(Saved {
            version,
            ..Saved::packed()
        });
        assert!(
            nmo::read(&file).is_ok(),
            "the reader takes version {version}"
        );
        let r = replace_pictures(&file, &one_picture(&rgba));
        assert!(
            matches!(r, Err(nmo::Error::UnsupportedVersion { file_version }) if file_version == version),
            "version {version}: {r:?}"
        );
    }
    let v9 = sample_saved(Saved {
        version: 9,
        ..Saved::packed()
    });
    assert!(replace_pictures(&v9, &one_picture(&rgba)).is_ok());
    for mode in [1, 9] {
        let old = sample_saved(Saved {
            write_mode: mode,
            ..Saved::packed()
        });
        assert!(
            replace_pictures(&old, &one_picture(&rgba)).is_err(),
            "chunk-compressed write mode {mode}"
        );
    }
}

#[test]
fn a_file_with_a_wrong_checksum_is_refused() {
    let rgba = pixels(3, 2, 34, false);
    for compress in [false, true] {
        let good = sample(compress);
        assert!(replace_pictures(&good, &one_picture(&rgba)).is_ok());
        for k in 8..12 {
            let mut bad = good.clone();
            bad[k] ^= 0x01;
            assert!(!nmo::read(&bad).unwrap().checksum_matches());
            assert!(
                replace_pictures(&bad, &one_picture(&rgba)).is_err(),
                "checksum byte {k} flipped"
            );
        }
    }
}

#[test]
fn parts_stored_differently_keep_header1_and_follow_the_write_mode() {
    let rgba = pixels(3, 2, 35, false);
    // header 1 packed, data plain, and the reverse, under either write mode
    for (pack_header1, pack_data) in [(true, false), (false, true)] {
        for write_mode in [0, 8] {
            let before = sample_saved(Saved {
                write_mode,
                pack_header1,
                pack_data,
                ..Saved::plain()
            });
            let after = replace_pictures(&before, &one_picture(&rgba)).unwrap();
            let (fb, fa) = (nmo::read(&before).unwrap(), nmo::read(&after).unwrap());
            assert!(fa.checksum_matches());
            assert_eq!(pictures(&after, 1)[0].2, rgba);
            // header 1 is copied as it was stored
            assert_eq!(header_u32(&after, 28), header_u32(&before, 28));
            let h1 = 64..64 + header_u32(&before, 28) as usize;
            assert_eq!(after[h1.clone()], before[h1]);
            assert_eq!(fa.header1, fb.header1);
            // the data part is packed exactly when the file was saved whole-compressed
            let (packed, unpacked) = (header_u32(&after, 32), header_u32(&after, 36));
            assert_eq!(
                packed < unpacked,
                write_mode == 8,
                "header1 packed {pack_header1}, data packed {pack_data}, mode {write_mode}"
            );
        }
    }
}
