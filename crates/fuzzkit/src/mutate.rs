// The mutations: input number `i` is a seed from the corpus changed by a few stacked mutations, all drawn from one
// xorshift generator seeded by (run seed, i), so any input can be made again from its number. Byte mutations for
// every format (flips, interesting values, inserts, deletes, copies, splices, truncations, length fields nudged);
// text formats also get mutations of their lines, words and numbers.

use crate::Kind;

/// xorshift64*: small, fixed and the same on every platform.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x2545_f491_4f6c_dd1d | 1)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// 0..n (0 when n is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }

    /// A length up to `max`, small ones much likelier.
    fn len(&mut self, max: usize) -> usize {
        let bits = 1 << self.below(13);
        (self.below(bits) + 1).min(max.max(1))
    }

    fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len())]
    }
}

const U32S: [u32; 40] = [
    0,
    1,
    2,
    3,
    4,
    7,
    8,
    15,
    16,
    31,
    32,
    63,
    64,
    127,
    128,
    255,
    256,
    511,
    512,
    1023,
    1024,
    4095,
    4096,
    65535,
    65536,
    0x00ff_ffff,
    0x0100_0000,
    0x3fff_ffff,
    0x7fff_ffff,
    0x8000_0000,
    0xffff_fffe,
    0xffff_ffff,
    0x7fc0_0000,
    0x7f80_0000,
    0xff80_0000,
    0x7f7f_ffff,
    0x0000_0001,
    0x7149_f2ca,
    0xbf80_0000,
    0x1000_0000,
];

const NUMBERS: [&str; 40] = [
    "0",
    "1",
    "-1",
    "2",
    "3",
    "255",
    "256",
    "65535",
    "65536",
    "2147483647",
    "2147483648",
    "-2147483648",
    "4294967295",
    "4294967296",
    "9223372036854775807",
    "9223372036854775808",
    "18446744073709551615",
    "18446744073709551616",
    "1e308",
    "1e309",
    "-1e309",
    "NaN",
    "nan",
    "inf",
    "-inf",
    "infinity",
    "0.0",
    "-0.0",
    "1e-45",
    "3.4028235e38",
    "0x10",
    "1.5",
    "007",
    "00000000000000000000001",
    "99999999999999999999999999999",
    "-0",
    "+1",
    "1,2,3",
    "1e",
    ".",
];

const SPECIAL: [&str; 34] = [
    "\0",
    "\r",
    "\r\n",
    "\n",
    "\t",
    " ",
    "\u{feff}",
    "\u{202e}",
    "\u{200b}",
    "é",
    "汉字",
    "\u{10ffff}",
    "#",
    ":",
    ";",
    "=",
    ",",
    "\"",
    "\\",
    "{",
    "}",
    "[",
    "]",
    "../",
    "/",
    "..",
    "-",
    "_",
    "\u{85}",
    "\u{2028}",
    "%s",
    "*",
    "~",
    "\u{1b}[",
];

pub struct Dict {
    pub words: Vec<Vec<u8>>,
    pub lines: Vec<Vec<u8>>,
}

impl Dict {
    /// Words and lines found in the seeds (text targets), plus the target's own words.
    pub fn new(corpus: &[Vec<u8>], extra: &[&str], kind: Kind) -> Dict {
        let mut words: Vec<Vec<u8>> = extra.iter().map(|w| w.as_bytes().to_vec()).collect();
        let mut lines = Vec::new();
        if kind == Kind::Text {
            let mut seen = std::collections::HashSet::new();
            for s in corpus {
                for line in s.split(|&b| b == b'\n').take(20_000) {
                    if lines.len() < 4096 && line.len() < 512 && seen.insert(line.to_vec()) {
                        lines.push(line.to_vec());
                    }
                    for w in line.split(|b| !is_word(*b)) {
                        if w.len() >= 2
                            && w.len() <= 64
                            && words.len() < 8192
                            && seen.insert(w.to_vec())
                        {
                            words.push(w.to_vec());
                        }
                    }
                }
            }
        }
        Dict { words, lines }
    }
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'+') || b >= 0x80
}

/// Input number `iter` of a run.
pub fn input(
    corpus: &[Vec<u8>],
    dict: &Dict,
    kind: Kind,
    seed: u64,
    iter: u64,
    max_len: usize,
) -> Vec<u8> {
    let mut rng = Rng::new(seed ^ iter.wrapping_mul(0x2127_599b_f432_5c37).rotate_left(17));
    let mut buf = rng.pick(corpus).clone();
    let stack = 1 << rng.below(4);
    let n = 1 + rng.below(stack);
    for _ in 0..n {
        if kind == Kind::Text && rng.below(4) != 0 {
            text(&mut buf, &mut rng, corpus, dict);
        } else {
            bytes(&mut buf, &mut rng, corpus);
        }
        buf.truncate(max_len);
    }
    buf
}

fn put(buf: &mut [u8], at: usize, v: &[u8]) {
    for (k, b) in v.iter().enumerate() {
        if let Some(slot) = buf.get_mut(at + k) {
            *slot = *b;
        }
    }
}

fn bytes(buf: &mut Vec<u8>, rng: &mut Rng, corpus: &[Vec<u8>]) {
    let len = buf.len();
    let pos = rng.below(len.max(1));
    match rng.below(16) {
        0 if len > 0 => buf[pos] ^= 1 << rng.below(8),
        1 if len > 0 => buf[pos] = rng.next() as u8,
        2 if len > 0 => buf[pos] = *rng.pick(&[0u8, 1, 0x7f, 0x80, 0xff, b'0', b' ', b'\n']),
        3 => {
            let v = *rng.pick(&U32S) as u16;
            let b = if rng.below(2) == 0 {
                v.to_le_bytes()
            } else {
                v.to_be_bytes()
            };
            put(buf, pos, &b);
        }
        4 | 5 => {
            // aligned words more often: the formats are made of them
            let at = if rng.below(2) == 0 { pos & !3 } else { pos };
            let v = *rng.pick(&U32S);
            let b = if rng.below(4) == 0 {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            };
            put(buf, at, &b);
        }
        6 => {
            let (hi, shift) = (u64::from(*rng.pick(&U32S)), 32 * rng.below(2));
            let v = hi << shift | u64::from(*rng.pick(&U32S));
            put(buf, pos, &v.to_le_bytes());
        }
        7 if len >= 4 => {
            let at = rng.below(len - 3);
            let v = u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
            let d = rng.below(64) as u32 + 1;
            let v = if rng.below(2) == 0 {
                v.wrapping_add(d)
            } else {
                v.wrapping_sub(d)
            };
            put(buf, at, &v.to_le_bytes());
        }
        8 => {
            let n = rng.len(64);
            let ins: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
            buf.splice(pos..pos, ins);
        }
        9 => {
            let n = rng.len(1 << 16);
            let b = if rng.below(2) == 0 {
                0
            } else {
                rng.next() as u8
            };
            buf.splice(pos..pos, std::iter::repeat_n(b, n));
        }
        10 if len > 0 => {
            let n = rng.len(len - pos);
            buf.drain(pos..pos + n);
        }
        11 if len > 0 => {
            // a stretch copied elsewhere, inserted or over what is there
            let from = rng.below(len);
            let n = rng.len(len - from);
            let piece = buf[from..from + n].to_vec();
            if rng.below(2) == 0 {
                buf.splice(pos..pos, piece);
            } else {
                put(buf, pos, &piece);
            }
        }
        12 => {
            // splice: this one's start, another's end
            let other = rng.pick(corpus);
            let from = rng.below(other.len().max(1));
            buf.truncate(pos);
            buf.extend_from_slice(other.get(from..).unwrap_or_default());
        }
        13 => {
            let other = rng.pick(corpus);
            if !other.is_empty() {
                let from = rng.below(other.len());
                let n = rng.len(other.len() - from);
                buf.splice(pos..pos, other[from..from + n].iter().copied());
            }
        }
        14 if len > 0 => buf.truncate(pos),
        _ => length_field(buf, rng),
    }
}

/// A word that reads as a size, a count or an offset (non-zero, at most the input's length) set a little or very
/// wrong: the edits that get past a parser's first checks.
fn length_field(buf: &mut [u8], rng: &mut Rng) {
    let len = buf.len();
    if len < 4 {
        return;
    }
    for _ in 0..64 {
        let at = rng.below(len / 4) * 4;
        let v = u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        if v == 0 || v as usize > len {
            continue;
        }
        let rest = (len - at) as u32;
        let new = match rng.below(10) {
            0 => v.wrapping_add(1),
            1 => v.wrapping_sub(1),
            2 => v.wrapping_mul(2),
            3 => v.wrapping_add(4),
            4 => 0,
            5 => u32::MAX,
            6 => 0x8000_0000,
            7 => rest,
            8 => rest.wrapping_add(1),
            _ => v.wrapping_add(0x10000),
        };
        put(buf, at, &new.to_le_bytes());
        return;
    }
    let at = rng.below(len / 4) * 4;
    put(buf, at, &rng.pick(&U32S).to_le_bytes());
}

/// (start, end) of the words of `buf`.
fn words(buf: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &b) in buf.iter().enumerate() {
        match (is_word(b), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, buf.len()));
    }
    out
}

/// (start, end without the newline) of the lines of `buf`.
fn lines(buf: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut s = 0;
    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' {
            out.push((s, i));
            s = i + 1;
        }
    }
    out.push((s, buf.len()));
    out
}

fn text(buf: &mut Vec<u8>, rng: &mut Rng, corpus: &[Vec<u8>], dict: &Dict) {
    let ws = words(buf);
    let ls = lines(buf);
    let word = |rng: &mut Rng| -> Option<(usize, usize)> {
        (!ws.is_empty()).then(|| ws[rng.below(ws.len())])
    };
    let line = |rng: &mut Rng| ls[rng.below(ls.len())];
    match rng.below(12) {
        0 => {
            if let Some((s, e)) = word(rng) {
                buf.splice(s..e, rng.pick(&NUMBERS).bytes());
            }
        }
        1 => {
            if let (Some((s, e)), false) = (word(rng), dict.words.is_empty()) {
                let w = rng.pick(&dict.words).clone();
                buf.splice(s..e, w);
            }
        }
        2 => {
            let at = word(rng).map_or(buf.len(), |(s, e)| if rng.below(2) == 0 { s } else { e });
            let mut w = if dict.words.is_empty() || rng.below(3) == 0 {
                rng.pick(&NUMBERS).as_bytes().to_vec()
            } else {
                rng.pick(&dict.words).clone()
            };
            if rng.below(2) == 0 {
                w.push(b' ');
            }
            buf.splice(at..at, w);
        }
        3 => {
            let (s, e) = line(rng);
            let mut l = buf[s..e].to_vec();
            l.push(b'\n');
            buf.splice(s..s, l);
        }
        4 => {
            let (s, e) = line(rng);
            buf.drain(s..(e + 1).min(buf.len()));
        }
        5 => {
            let (a, b) = (line(rng), line(rng));
            let (first, second) = if a.0 <= b.0 { (a, b) } else { (b, a) };
            if first.1 < second.0 {
                let (x, y) = (
                    buf[first.0..first.1].to_vec(),
                    buf[second.0..second.1].to_vec(),
                );
                buf.splice(second.0..second.1, x);
                buf.splice(first.0..first.1, y);
            }
        }
        6 => {
            let at = line(rng).0;
            let mut l = if dict.lines.is_empty() {
                let o = rng.pick(corpus);
                let ol = lines(o);
                let (s, e) = ol[rng.below(ol.len())];
                o[s..e].to_vec()
            } else {
                rng.pick(&dict.lines).clone()
            };
            l.push(b'\n');
            buf.splice(at..at, l);
        }
        7 => {
            let (s, e) = line(rng);
            let mut l = buf[s..e].to_vec();
            l.push(b'\n');
            let n = 1usize << rng.below(12);
            let rep: Vec<u8> = std::iter::repeat_n(l, n).flatten().take(1 << 20).collect();
            buf.splice(s..s, rep);
        }
        8 => {
            let at = rng.below(buf.len() + 1);
            let s = if rng.below(8) == 0 {
                vec![b'A'; rng.len(1 << 16)]
            } else if rng.below(8) == 0 {
                vec![0xff, 0xfe, 0xc3]
            } else {
                rng.pick(&SPECIAL).as_bytes().to_vec()
            };
            buf.splice(at..at, s);
        }
        9 => {
            if let Some((s, e)) = word(rng) {
                let w = buf[s..e].to_vec();
                let new: Vec<u8> = match rng.below(5) {
                    0 => [b"-".as_slice(), &w].concat(),
                    1 => [w.as_slice(), b"000000000"].concat(),
                    2 => [w.as_slice(), b"e999"].concat(),
                    3 => w
                        .iter()
                        .map(|c| if c.is_ascii_digit() { b'9' } else { *c })
                        .collect(),
                    _ => w.to_ascii_uppercase(),
                };
                buf.splice(s..e, new);
            }
        }
        10 => {
            let at = word(rng).map_or(0, |w| w.0);
            let n = rng.len(1 << 12);
            let open: &[u8] = rng.pick(&[b"[".as_slice(), b"{\"a\":", b"(", b"\""]);
            let rep: Vec<u8> = std::iter::repeat_n(open, n).flatten().copied().collect();
            buf.splice(at..at, rep);
        }
        _ => {
            if let Some((s, e)) = word(rng) {
                buf.drain(s..e);
            }
        }
    }
}
