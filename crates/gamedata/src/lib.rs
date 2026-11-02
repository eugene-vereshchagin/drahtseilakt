// The player's copy of the game and the level folders, shared by the game (crates/game) and drahtseilakt-toolbox
// (its own repository, which takes this crate by path): where the copy and the levels are, finding files and textures in the copy, the level table, and
// loading and exporting levels.

pub mod assets;
pub mod profile;

/// FNV-1a, 64 bits: the hash a level file is known by (replays name the level file they were recorded on).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}
