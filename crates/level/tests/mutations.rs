// Mutation fuzzing over the player's own files (OPENBALLANCE_DATA; skipped without it): each file's container is
// read once, then its decompressed parts are damaged again and again (bytes flipped, words set to 0, -1 and other
// edge values) and run through the whole path a level takes: the object table, every scene decoder on every object
// whatever its class, and the level model. Nothing may panic; errors and warnings are the right answer. Damaging
// the decompressed parts reaches the decoders, which damage to the compressed stream mostly would not.
// MUTATION_ITERS sets the rounds a file (default 300); MUTATION_SEED replays one round's damage, MUTATION_FILE
// keeps the files whose name contains it, and MUTATION_TRACE prints each round before it runs (an abort, which
// unwinding cannot catch, then still shows its seed).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use nmo::{Document, scene};

/// xorshift64*: a small, fixed generator so a failing round can be replayed from its seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const EDGES: [u32; 8] = [0, 1, 0xffff_ffff, 0x7fff_ffff, 0x8000_0000, 0xffff, 0x1_0000, 0x40];

/// Damages `part` a few times: a flipped byte, or an aligned word set to an edge value.
fn damage(part: &mut [u8], rng: &mut Rng) {
    if part.is_empty() {
        return;
    }
    for _ in 0..1 + rng.below(8) {
        if rng.below(2) == 0 {
            let i = rng.below(part.len());
            part[i] ^= 1 << rng.below(8);
        } else if part.len() >= 4 {
            let i = rng.below(part.len() / 4) * 4;
            let v = EDGES[rng.below(EDGES.len())];
            part[i..i + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
}

/// Every decoder on every object: a damaged class id or chunk must not panic any of them.
fn decode_all(doc: &Document) {
    for o in &doc.objects {
        let Some(c) = &o.chunk else { continue };
        let _ = scene::mesh(c);
        let _ = scene::mesh_channels(c, scene::mesh(c).map_or(64, |m| m.positions.len()));
        let _ = scene::material(c);
        let _ = scene::texture(c);
        let _ = scene::entity(c);
        let _ = scene::curve(c);
        let _ = scene::curve_point(c);
        let _ = scene::camera(c);
        let _ = scene::sprite(c);
        let _ = scene::material_effect(c);
        let _ = scene::parameter_word(c);
        let _ = scene::group(c);
        let _ = scene::data_array(c);
        let _ = scene::object_animation(c);
    }
}

fn round(file: &nmo::File, seed: u64) {
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut f = file.clone();
    // the object table's damage mostly hits names, which are refused early: it gets an eighth of the rounds
    match rng.below(8) {
        0 => damage(&mut f.header1, &mut rng),
        _ => damage(&mut f.data, &mut rng),
    }
    if let Ok(doc) = Document::parse(&f) {
        decode_all(&doc);
    }
    let _ = level::from_nmo::load_file(&f);
}

#[test]
fn damaged_game_files_never_panic() {
    let Ok(root) = std::env::var("OPENBALLANCE_DATA") else {
        eprintln!("OPENBALLANCE_DATA not set; skipped");
        return;
    };
    let iters: u64 = std::env::var("MUTATION_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let only: Option<u64> = std::env::var("MUTATION_SEED").ok().and_then(|v| v.parse().ok());
    let filter = std::env::var("MUTATION_FILE").unwrap_or_default();
    let trace = std::env::var_os("MUTATION_TRACE").is_some();
    let files = [
        "3D Entities/Level/Level_01.NMO",
        "3D Entities/Level/Level_07.NMO",
        "3D Entities/Menu.nmo",
        "3D Entities/Levelinit.nmo",
        "3D Entities/Language.nmo",
        "3D Entities/Balls.nmo",
        "3D Entities/MenuLevel.nmo",
    ];
    for (k, name) in files.iter().enumerate() {
        if !name.contains(filter.as_str()) {
            continue;
        }
        let path = format!("{root}/{name}");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("{path}: not in this copy");
            continue;
        };
        let file = nmo::read(&bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
        let seeds: Vec<u64> = match only {
            Some(s) => vec![s],
            None => (0..iters).map(|i| ((k as u64) << 32) ^ (i * 0x9e37_79b9) ^ 0x5eed).collect(),
        };
        for seed in seeds {
            if trace {
                eprintln!("{name}: MUTATION_SEED={seed}");
            }
            if std::panic::catch_unwind(|| round(&file, seed)).is_err() {
                panic!("{name}: panicked on MUTATION_FILE={name:?} MUTATION_SEED={seed}");
            }
        }
        eprintln!("{name}: {} rounds", if only.is_some() { 1 } else { iters });
    }
}
