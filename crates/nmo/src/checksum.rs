// The file checksum. Virtools stores an Adler-32 that starts from 0 instead of the standard 1, taken over the
// 64-byte header with its checksum field zeroed and then over the rest of the file. Found by testing the
// original files (all 13 levels and base.cmo match); written here so the reader can refuse damaged files.

const MOD: u32 = 65_521;
// The largest run of bytes whose sums cannot overflow u32 before the modulo (as in zlib).
const NMAX: usize = 5_552;

#[derive(Clone, Copy)]
pub struct Adler32 {
    a: u32,
    b: u32,
}

impl Adler32 {
    /// The Virtools variant: both halves start at zero.
    pub fn virtools() -> Self {
        Adler32 { a: 0, b: 0 }
    }

    pub fn update(&mut self, data: &[u8]) {
        for chunk in data.chunks(NMAX) {
            for &byte in chunk {
                // Bounded by NMAX: a and b stay below 2^32 within one chunk.
                self.a = self.a.wrapping_add(u32::from(byte));
                self.b = self.b.wrapping_add(self.a);
            }
            self.a %= MOD;
            self.b %= MOD;
        }
    }

    pub fn finish(self) -> u32 {
        (self.b << 16) | self.a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_adler_shifted_by_seed() {
        // Seeded with 1 the algorithm is the standard Adler-32: "Wikipedia" is 0x11E60398.
        let mut s = Adler32 { a: 1, b: 0 };
        s.update(b"Wikipedia");
        assert_eq!(s.finish(), 0x11E6_0398);
    }

    #[test]
    fn long_input_stays_in_range() {
        let data = vec![0xFFu8; 100_000];
        let mut s = Adler32::virtools();
        s.update(&data);
        let v = s.finish();
        assert!((v & 0xFFFF) < MOD && (v >> 16) < MOD);
    }
}
