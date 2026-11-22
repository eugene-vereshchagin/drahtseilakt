// Where the physics' step grid stands when a level of the original starts.

/// The start phase of the original's levels 1 to 12, by the hash of their files (a converted level carries it as its
/// `source_hash`), in tenths of a PSI. Measured on the original at 60 frames a second (runs 8 to 11, 2026-11-23): its
/// physics clock runs while a level loads, 1.1 PSIs a frame, and the load takes each level the same number of frames
/// (levels 2 to 12: 102, 106, 105, 112, 106, 102, 107, 102, 107, 112, 139; level 1 starts longer).
const TENTHS: [(u64, u8); 12] = [
    (0xaa6b_96ea_7024_869c, 3),
    (0x4d0e_1756_916b_22e1, 0),
    (0xf335_d402_50a5_a5f3, 4),
    (0x9350_8706_3a01_dfde, 3),
    (0x0e57_f9df_0328_de3c, 0),
    (0xb9f2_ec13_ebc0_2e0b, 4),
    (0xdae6_e38f_5604_f0ca, 0),
    (0x09ee_8657_8670_7dc8, 5),
    (0x03cc_4079_fba7_71ed, 0),
    (0xc244_377f_173d_7424, 5),
    (0xaf3d_3e71_9324_4751, 0),
    (0x4029_bb66_9ed3_1a9a, 7),
];

/// What each measured phase has over its tenth (0.000031 to 0.000050 in the fits to the original's free fall).
const OVER: f64 = 0.000_042;

/// The phase, as a part of a PSI, a level made from the file with this hash starts at: 0 for a level not measured.
pub fn start_phase(source_hash: Option<u64>) -> f64 {
    TENTHS
        .iter()
        .find(|(h, _)| Some(*h) == source_hash)
        .map_or(0.0, |(_, t)| f64::from(*t) / 10.0 + OVER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_levels_start_at_their_phase_and_others_at_zero() {
        assert_eq!(start_phase(None), 0.0);
        assert_eq!(start_phase(Some(1)), 0.0);
        assert!((start_phase(Some(0xaa6b_96ea_7024_869c)) - 0.300_042).abs() < 1e-12);
        assert!((start_phase(Some(0x4029_bb66_9ed3_1a9a)) - 0.700_042).abs() < 1e-12);
    }
}
