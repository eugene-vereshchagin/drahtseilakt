// The original's pooled module copies and their wake-up countdowns at a level's start, for levels 1 to 12.
//
// The original makes a few copies of each module (its own object, then 001, 002, ...) and gives each placeholder of
// the level one of them in its PH array, so placeholders of different sectors share a copy. A copy's wake-up test
// (TT Scaleable Proximity) keeps its countdown to the next test across sectors: it stops while the copy is out of
// the physics and goes on where it was when the copy comes in again. At the level's start each copy's countdown is
// where the level's load left it. Both were read on the original (TAS runs, 2026-11-23: the PH array's Object
// column and the blocks' "Last Check" at the level's first frame; NOTES/tas-results-13-idle, -23-prox12). The fans
// (P_Modul_18) are not here: their test switches the updraft, not a wake-up. A level not listed, or a placeholder
// not listed, has a copy of its own whose first test comes at once.

/// (placeholder, copy: 0 the module's own object, 1 its copy 001, ..., countdown at the level's first frame).
pub type Copy = (&'static str, u8, u8);

const TABLE: [(u64, &[Copy]); 12] = [
    // level 1
    (
        0xaa6b_96ea_7024_869c,
        &[
            ("P_Modul_01_01", 0, 54),
            ("P_Modul_01_02", 1, 54),
            ("P_Modul_34_01", 0, 54),
        ],
    ),
    // level 2
    (
        0x4d0e_1756_916b_22e1,
        &[
            ("P_Modul_01_03", 0, 47),
            ("P_Modul_25_01", 0, 49),
            ("P_Modul_29_01", 0, 48),
            ("P_Modul_30_01", 0, 48),
        ],
    ),
    // level 3
    (
        0xf335_d402_50a5_a5f3,
        &[
            ("P_Modul_25_03", 1, 46),
            ("P_Modul_25_04", 1, 46),
            ("P_Modul_25_05", 0, 46),
            ("P_Modul_29_01", 0, 56),
            ("P_Modul_30_01", 0, 56),
            ("P_Modul_37_01", 0, 48),
        ],
    ),
    // level 4
    (
        0x9350_8706_3a01_dfde,
        &[("P_Modul_25_02", 0, 45), ("P_Modul_29_02", 0, 44)],
    ),
    // level 5
    (
        0x0e57_f9df_0328_de3c,
        &[
            ("P_Modul_01_01", 1, 55),
            ("P_Modul_01_02", 1, 55),
            ("P_Modul_01_03", 1, 55),
            ("P_Modul_01_05", 0, 51),
            ("P_Modul_01_06", 0, 51),
            ("P_Modul_25_01", 0, 45),
            ("P_Modul_30_03", 0, 44),
            ("P_Modul_34_01", 1, 50),
            ("P_Modul_34_02", 0, 46),
            ("P_Modul_34_03", 2, 46),
            ("P_Modul_34_04", 1, 50),
            ("P_Modul_37_01", 0, 54),
            ("P_Modul_37_02", 0, 54),
            ("P_Modul_37_03", 0, 54),
        ],
    ),
    // level 6
    (
        0xb9f2_ec13_ebc0_2e0b,
        &[
            ("P_Modul_25_01", 0, 55),
            ("P_Modul_29_01", 1, 41),
            ("P_Modul_29_02", 0, 41),
        ],
    ),
    // level 7
    (
        0xdae6_e38f_5604_f0ca,
        &[
            ("P_Modul_01_01", 0, 52),
            ("P_Modul_01_02", 0, 52),
            ("P_Modul_03_01", 0, 45),
            ("P_Modul_25_01", 0, 48),
            ("P_Modul_25_02", 0, 48),
            ("P_Modul_29_01", 1, 56),
            ("P_Modul_29_02", 0, 55),
            ("P_Modul_29_03", 1, 56),
            ("P_Modul_30_01", 1, 49),
            ("P_Modul_30_02", 0, 52),
            ("P_Modul_34_01", 2, 51),
            ("P_Modul_34_02", 1, 49),
            ("P_Modul_34_03", 0, 49),
            ("P_Modul_34_04", 2, 51),
            ("P_Modul_37_01", 0, 55),
        ],
    ),
    // level 8
    (
        0x09ee_8657_8670_7dc8,
        &[
            ("P_Modul_01_01", 0, 52),
            ("P_Modul_25_01", 1, 53),
            ("P_Modul_25_02", 1, 53),
            ("P_Modul_25_03", 0, 50),
            ("P_Modul_25_04", 0, 50),
            ("P_Modul_29_01", 0, 53),
            ("P_Modul_29_02", 0, 53),
            ("P_Modul_29_03", 0, 53),
            ("P_Modul_30_01", 0, 57),
            ("P_Modul_30_02", 0, 57),
            ("P_Modul_30_03", 0, 57),
            ("P_Modul_37_01", 0, 46),
            ("P_Modul_37_02", 1, 46),
            ("P_Modul_37_03", 1, 46),
        ],
    ),
    // level 9
    (
        0x03cc_4079_fba7_71ed,
        &[
            ("P_Modul_01_01", 0, 53),
            ("P_Modul_01_02", 1, 53),
            ("P_Modul_01_03", 1, 53),
            ("P_Modul_01_04", 0, 53),
            ("P_Modul_03_01", 1, 44),
            ("P_Modul_03_02", 2, 43),
            ("P_Modul_03_03", 0, 43),
            ("P_Modul_19_01", 0, 32),
            ("P_Modul_19_02", 1, 57),
            ("P_Modul_25_01", 0, 45),
            ("P_Modul_25_02", 0, 45),
            ("P_Modul_29_01", 0, 50),
            ("P_Modul_29_02", 0, 50),
            ("P_Modul_30_01", 0, 51),
            ("P_Modul_30_02", 0, 51),
            ("P_Modul_37_01", 0, 49),
            ("P_Modul_37_02", 0, 49),
        ],
    ),
    // level 10
    (
        0xc244_377f_173d_7424,
        &[
            ("P_Modul_01_01", 1, 51),
            ("P_Modul_01_02", 0, 55),
            ("P_Modul_01_03", 1, 51),
            ("P_Modul_01_04", 0, 55),
            ("P_Modul_03_01", 0, 49),
            ("P_Modul_19_01", 0, 44),
            ("P_Modul_25_01", 1, 44),
            ("P_Modul_25_02", 2, 44),
            ("P_Modul_25_03", 0, 44),
            ("P_Modul_25_04", 1, 44),
            ("P_Modul_29_01", 0, 49),
            ("P_Modul_29_02", 1, 44),
            ("P_Modul_29_03", 0, 49),
            ("P_Modul_30_01", 0, 49),
            ("P_Modul_30_02", 0, 49),
            ("P_Modul_30_03", 0, 49),
            ("P_Modul_34_01", 0, 51),
            ("P_Modul_34_02", 1, 57),
            ("P_Modul_34_03", 2, 57),
            ("P_Modul_37_01", 1, 57),
            ("P_Modul_37_02", 0, 49),
            ("P_Modul_37_03", 0, 49),
            ("P_Modul_37_04", 1, 57),
        ],
    ),
    // level 11
    (
        0xaf3d_3e71_9324_4751,
        &[
            ("P_Modul_01_01", 0, 57),
            ("P_Modul_01_02", 1, 57),
            ("P_Modul_01_03", 1, 57),
            ("P_Modul_01_04", 0, 57),
            ("P_Modul_01_05", 1, 57),
            ("P_Modul_19_01", 1, 52),
            ("P_Modul_19_02", 0, 55),
            ("P_Modul_19_03", 0, 55),
            ("P_Modul_19_04", 1, 52),
            ("P_Modul_25_01", 1, 56),
            ("P_Modul_25_02", 0, 56),
            ("P_Modul_25_03", 0, 56),
            ("P_Modul_25_04", 1, 56),
            ("P_Modul_25_05", 0, 56),
            ("P_Modul_25_06", 1, 56),
            ("P_Modul_25_07", 0, 56),
            ("P_Modul_29_01", 0, 49),
            ("P_Modul_30_01", 1, 54),
            ("P_Modul_30_02", 0, 48),
            ("P_Modul_30_03", 0, 48),
            ("P_Modul_30_04", 1, 54),
            ("P_Modul_34_01", 1, 54),
            ("P_Modul_34_02", 2, 48),
            ("P_Modul_34_03", 3, 48),
            ("P_Modul_34_04", 4, 47),
            ("P_Modul_34_05", 5, 47),
            ("P_Modul_34_06", 0, 47),
            ("P_Modul_34_07", 1, 54),
            ("P_Modul_37_01", 0, 57),
            ("P_Modul_37_02", 0, 57),
            ("P_Modul_37_03", 0, 57),
            ("P_Modul_37_04", 0, 57),
        ],
    ),
    // level 12
    (
        0x4029_bb66_9ed3_1a9a,
        &[
            ("P_Modul_03_01", 0, 39),
            ("P_Modul_03_02", 3, 39),
            ("P_Modul_03_03", 2, 39),
            ("P_Modul_03_04", 1, 39),
            ("P_Modul_25_01", 0, 49),
            ("P_Modul_25_02", 1, 49),
            ("P_Modul_25_03", 2, 43),
            ("P_Modul_37_01", 0, 51),
            ("P_Modul_37_02", 2, 37),
            ("P_Modul_37_03", 0, 51),
            ("P_Modul_37_04", 1, 38),
        ],
    ),
];

/// The placeholders' copies of a level of the original, by the hash of its file (as `startphase`).
pub fn copies(source_hash: Option<u64>) -> &'static [Copy] {
    TABLE
        .iter()
        .find(|(h, _)| Some(*h) == source_hash)
        .map_or(&[], |(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_have_their_copies_and_others_none() {
        // level 5: the pusher of sector 1 and one of sector 2 share copy 001 (the s2pass scene's wake at frame 427)
        let l5 = copies(Some(0x0e57_f9df_0328_de3c));
        assert!(l5.contains(&("P_Modul_01_01", 1, 55)));
        assert!(l5.contains(&("P_Modul_01_03", 1, 55)));
        assert!(l5.contains(&("P_Modul_01_06", 0, 51)));
        assert!(copies(Some(1)).is_empty());
        assert!(copies(None).is_empty());
    }
}
