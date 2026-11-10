// Textures ready for the graphics card (docs/drahtseilakt/open-format.md, "Compressed textures"): the `.dtx` file
// the toolbox writes beside a level's large picture, holding it block-compressed (BC1 or BC7) with every mip level
// made beforehand, so the game neither decodes a PNG nor filters mips when the level loads, and the picture takes
// an eighth or a quarter of the video memory. The file comes with a level folder, so it is a stranger's: every field
// is checked against the caps before anything is allocated or read, the length must be exactly what the header
// implies, the data's hash must match, and a file that fails any of it is refused whole.
// The decoders here turn the blocks back into RGBA for an adapter without BC and for the tests.

/// The file's first bytes and the version this reader knows.
pub const MAGIC: [u8; 4] = *b"DSTX";
pub const VERSION: u16 = 1;
/// The header's length: magic, version, format, mip count, width, height, the source file's length and hash, the
/// data's hash.
pub const HEADER_LEN: usize = 40;
/// The largest side (as the PNG reader's) and so the largest file: a 4096 x 4096 BC7 chain is under 23 MB.
pub const SIDE_MAX: u32 = 4096;
pub const FILE_MAX: u64 = 32 * 1024 * 1024;

/// How the pixels are stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Format {
    /// Plain RGBA, 4 bytes a pixel (never in a file: what a decoded picture is).
    Rgba,
    /// BC1 (4 bits a pixel), opaque: every block in its four-colour mode or with equal endpoints.
    Bc1,
    /// BC7 (8 bits a pixel) with alpha: every block of mode 6.
    Bc7,
}

impl Format {
    fn code(self) -> u8 {
        match self {
            Format::Rgba => 0,
            Format::Bc1 => 1,
            Format::Bc7 => 2,
        }
    }

    /// Bytes of one 4 x 4 block (of one pixel for RGBA).
    pub fn block_bytes(self) -> usize {
        match self {
            Format::Rgba => 4,
            Format::Bc1 => 8,
            Format::Bc7 => 16,
        }
    }

    /// The bytes of a `width` x `height` mip level.
    pub fn level_bytes(self, width: u32, height: u32) -> Option<usize> {
        let (w, h) = match self {
            Format::Rgba => (width, height),
            _ => (width.div_ceil(4), height.div_ceil(4)),
        };
        usize::try_from(w)
            .ok()?
            .checked_mul(usize::try_from(h).ok()?)?
            .checked_mul(self.block_bytes())
    }
}

/// One mip level: where its bytes are in the data, and its size in pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mip {
    pub offset: usize,
    pub len: usize,
    pub width: u32,
    pub height: u32,
}

/// A texture as the renderer uploads it: its mip levels, the largest first, in one buffer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TextureData {
    pub format: Format,
    pub data: Vec<u8>,
    pub mips: Vec<Mip>,
    /// The data's hash when it came from a `.dtx` file (checked against the data): two textures of one hash, format
    /// and size are one picture, uploaded once.
    pub hash: Option<u64>,
}

impl TextureData {
    /// The size of the largest level.
    pub fn size(&self) -> (u32, u32) {
        self.mips.first().map_or((0, 0), |m| (m.width, m.height))
    }

    /// The bytes all the levels take (in video memory as in the buffer).
    pub fn bytes(&self) -> usize {
        self.mips
            .iter()
            .fold(0usize, |n, m| n.saturating_add(m.len))
    }

    /// A level's bytes.
    pub fn level(&self, m: &Mip) -> Option<&[u8]> {
        self.data.get(m.offset..m.offset.checked_add(m.len)?)
    }

    /// RGBA pixels with their whole mip chain, box-filtered (as the renderer has always made it).
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<TextureData> {
        let first = Format::Rgba.level_bytes(width, height)?;
        if width == 0 || height == 0 || rgba.len() != first {
            return None;
        }
        let mut mips = vec![Mip {
            offset: 0,
            len: first,
            width,
            height,
        }];
        let mut data = rgba;
        let (mut w, mut h, mut at) = (width, height, 0usize);
        while w > 1 || h > 1 {
            let next = halve(w, h, data.get(at..)?)?;
            (w, h) = ((w / 2).max(1), (h / 2).max(1));
            at = data.len();
            mips.push(Mip {
                offset: at,
                len: next.len(),
                width: w,
                height: h,
            });
            data.extend_from_slice(&next);
        }
        Some(TextureData {
            format: Format::Rgba,
            data,
            mips,
            hash: None,
        })
    }

    /// The same texture without its `n` largest levels (at least one is kept).
    pub fn without_top(mut self, n: usize) -> TextureData {
        let n = n.min(self.mips.len().saturating_sub(1));
        self.mips.drain(..n);
        self
    }
}

/// An RGBA picture at half the size (each side at least 1): every pixel the mean of the four it covers, the last
/// row or column taken twice where a side is odd.
pub fn halve(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = (usize::try_from(width).ok()?, usize::try_from(height).ok()?);
    let row = w.checked_mul(4)?;
    if w == 0 || h == 0 || rgba.len() < row.checked_mul(h)? {
        return None;
    }
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = Vec::with_capacity(nw.checked_mul(nh)?.checked_mul(4)?);
    for y in 0..nh {
        let y0 = y.saturating_mul(2).min(h.saturating_sub(1));
        let y1 = y
            .saturating_mul(2)
            .saturating_add(1)
            .min(h.saturating_sub(1));
        let top = rgba.get(y0.checked_mul(row)?..)?.get(..row)?;
        let bottom = rgba.get(y1.checked_mul(row)?..)?.get(..row)?;
        if w >= 2 {
            // each pair of pixels of the two rows (the caller's sides are powers of two as a rule; an odd side's last
            // column is left out exactly as the box of the pixels before it leaves it)
            for (a, b) in top
                .as_chunks::<8>()
                .0
                .iter()
                .zip(bottom.as_chunks::<8>().0)
                .take(nw)
            {
                for c in 0..4usize {
                    let s = [
                        a.get(c),
                        a.get(c.wrapping_add(4)),
                        b.get(c),
                        b.get(c.wrapping_add(4)),
                    ]
                    .into_iter()
                    .flatten()
                    .fold(0u32, |s, &v| s.wrapping_add(u32::from(v)));
                    out.push(u8::try_from(s / 4).unwrap_or(u8::MAX));
                }
            }
        } else {
            for c in 0..4usize {
                let s = u32::from(top.get(c).copied()?)
                    .wrapping_add(u32::from(bottom.get(c).copied()?));
                out.push(u8::try_from(s / 2).unwrap_or(u8::MAX));
            }
        }
    }
    Some(out)
}

/// The hash of a file's data: FNV-1a over its 8-byte words (the tail padded with zeros), then its length. Fast
/// enough to check a level's every texture as it loads; it tells a damaged or cut file, not a forged one.
pub fn data_hash(bytes: &[u8]) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let (words, tail) = bytes.as_chunks::<8>();
    let mut h = words.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, w| {
        (h ^ u64::from_le_bytes(*w)).wrapping_mul(PRIME)
    });
    if !tail.is_empty() {
        let mut last = [0u8; 8];
        for (d, s) in last.iter_mut().zip(tail) {
            *d = *s;
        }
        h = (h ^ u64::from_le_bytes(last)).wrapping_mul(PRIME);
    }
    (h ^ u64::try_from(bytes.len()).unwrap_or(u64::MAX)).wrapping_mul(PRIME)
}

/// What a `.dtx` file says of itself, every field within the caps.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Header {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    /// The length and hash (`data_hash`) of the picture file it was made from: the game compares the length with the
    /// PNG beside it (a picture changed since is not the file's), the toolbox both.
    pub source_len: u64,
    pub source_hash: u64,
    /// The hash of the data after the header.
    pub data_hash: u64,
    /// The mip levels, their offsets counted from the end of the header.
    pub mips: Vec<Mip>,
}

impl Header {
    /// The bytes after the header.
    pub fn data_len(&self) -> usize {
        self.mips
            .iter()
            .fold(0usize, |n, m| n.saturating_add(m.len))
    }

    /// The whole file's length.
    pub fn file_len(&self) -> u64 {
        u64::try_from(self.data_len().saturating_add(HEADER_LEN)).unwrap_or(u64::MAX)
    }

    /// How many of the largest levels can be left out (the level then first must still be whole blocks: both sides
    /// multiples of four).
    pub fn droppable(&self) -> usize {
        self.mips
            .iter()
            .skip(1)
            .take_while(|m| m.width.is_multiple_of(4) && m.height.is_multiple_of(4))
            .count()
    }

    /// The bytes of the levels from `skip` on.
    pub fn bytes_from(&self, skip: usize) -> usize {
        self.mips
            .iter()
            .skip(skip)
            .fold(0usize, |n, m| n.saturating_add(m.len))
    }
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(at..)?.first_chunk::<2>().copied()?,
    ))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..)?.first_chunk::<4>().copied()?,
    ))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        b.get(at..)?.first_chunk::<8>().copied()?,
    ))
}

/// The mip chain of a `width` x `height` texture: `count` levels, each half the one before.
fn chain(format: Format, width: u32, height: u32, count: usize) -> Option<Vec<Mip>> {
    let mut mips = Vec::with_capacity(count.min(16));
    let (mut w, mut h, mut offset) = (width, height, 0usize);
    for _ in 0..count {
        let len = format.level_bytes(w, h)?;
        mips.push(Mip {
            offset,
            len,
            width: w,
            height: h,
        });
        offset = offset.checked_add(len)?;
        (w, h) = ((w / 2).max(1), (h / 2).max(1));
    }
    Some(mips)
}

/// How many levels the whole chain of a texture has (down to 1 x 1).
pub fn full_chain(width: u32, height: u32) -> usize {
    let side = width.max(height).max(1);
    usize::try_from(side.ilog2()).unwrap_or(0).saturating_add(1)
}

/// Reads a file's header (its first HEADER_LEN bytes are enough) and checks every field: the magic and version, a
/// format we know, sides that are multiples of four within the cap, and the whole mip chain.
pub fn header(bytes: &[u8]) -> Result<Header, String> {
    if bytes.get(..4) != Some(&MAGIC[..]) {
        return Err("not a compressed texture".into());
    }
    let short = || "the header is cut short".to_owned();
    let version = u16_at(bytes, 4).ok_or_else(short)?;
    if version != VERSION {
        return Err(format!("version {version}: this game reads {VERSION}"));
    }
    let format = match bytes.get(6).copied().ok_or_else(short)? {
        1 => Format::Bc1,
        2 => Format::Bc7,
        f => return Err(format!("format {f} unknown")),
    };
    let count = usize::from(bytes.get(7).copied().ok_or_else(short)?);
    let width = u32_at(bytes, 8).ok_or_else(short)?;
    let height = u32_at(bytes, 12).ok_or_else(short)?;
    let whole = |s: u32| (4..=SIDE_MAX).contains(&s) && s.is_multiple_of(4);
    if !whole(width) || !whole(height) {
        return Err(format!(
            "{width} x {height}: sides are multiples of 4 up to {SIDE_MAX}"
        ));
    }
    // every level down to 1 x 1: the renderer never has to make one
    if count != full_chain(width, height) {
        return Err(format!("{count} mip levels: not the whole chain"));
    }
    let mips = chain(format, width, height, count).ok_or("the mip chain is too big")?;
    Ok(Header {
        format,
        width,
        height,
        source_len: u64_at(bytes, 16).ok_or_else(short)?,
        source_hash: u64_at(bytes, 24).ok_or_else(short)?,
        data_hash: u64_at(bytes, 32).ok_or_else(short)?,
        mips,
    })
}

/// Reads a whole file: the header, the length exactly what it implies, the data's hash, and every block of a kind
/// this format allows (so the card and the decoder here always show the same picture).
pub fn read(bytes: &[u8]) -> Result<Header, String> {
    let h = header(bytes)?;
    let data = bytes.get(HEADER_LEN..).ok_or("the header is cut short")?;
    if data.len() != h.data_len() {
        return Err(format!(
            "{} bytes of data, the header says {}",
            data.len(),
            h.data_len()
        ));
    }
    if data_hash(data) != h.data_hash {
        return Err("the data does not match its hash".into());
    }
    let allowed = match h.format {
        Format::Bc7 => data.as_chunks::<16>().0.iter().all(|b| b[0] & 0x7f == 0x40),
        // four-colour mode (the first colour the larger), or one colour with every index 0 or 1 (never the
        // three-colour mode's transparent black)
        Format::Bc1 => data.as_chunks::<8>().0.iter().all(|b| {
            let (c0, c1) = (
                u16::from_le_bytes([b[0], b[1]]),
                u16::from_le_bytes([b[2], b[3]]),
            );
            c0 > c1 || (c0 == c1 && b[4..].iter().all(|i| i & 0xaa == 0))
        }),
        Format::Rgba => false,
    };
    if !allowed {
        return Err("a block of a mode this format does not use".into());
    }
    Ok(h)
}

/// A whole file read into a texture, without its `skip` largest levels (as many as `Header::droppable` allows).
pub fn texture(bytes: Vec<u8>, skip: usize) -> Result<TextureData, String> {
    let h = read(&bytes)?;
    let skip = skip.min(h.droppable());
    let mips = h
        .mips
        .iter()
        .skip(skip)
        .map(|m| {
            Some(Mip {
                offset: m.offset.checked_add(HEADER_LEN)?,
                ..*m
            })
        })
        .collect::<Option<Vec<Mip>>>()
        .ok_or("the mip chain is too big")?;
    Ok(TextureData {
        format: h.format,
        data: bytes,
        mips,
        hash: Some(h.data_hash),
    })
}

/// Writes a file: `levels` are the whole mip chain's blocks, the largest first.
pub fn write(
    format: Format,
    width: u32,
    height: u32,
    source: (u64, u64),
    levels: &[Vec<u8>],
) -> Result<Vec<u8>, String> {
    let mips = chain(format, width, height, levels.len()).ok_or("the mip chain is too big")?;
    if format == Format::Rgba
        || levels.len() != full_chain(width, height)
        || mips.iter().zip(levels).any(|(m, l)| m.len != l.len())
    {
        return Err("not a whole mip chain of blocks".into());
    }
    let data: Vec<u8> = levels.concat();
    let mut out = Vec::with_capacity(data.len().saturating_add(HEADER_LEN));
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.push(format.code());
    out.push(u8::try_from(levels.len()).map_err(|_| "too many mip levels")?);
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&source.0.to_le_bytes());
    out.extend_from_slice(&source.1.to_le_bytes());
    out.extend_from_slice(&data_hash(&data).to_le_bytes());
    out.extend_from_slice(&data);
    // what is written is what the reader takes
    read(&out)?;
    Ok(out)
}

/// A texture's blocks decoded into RGBA with the same mip levels (for an adapter without BC, and the tests).
pub fn decoded(t: &TextureData) -> Option<TextureData> {
    if t.format == Format::Rgba {
        return Some(t.clone());
    }
    let mut data = Vec::new();
    let mut mips = Vec::with_capacity(t.mips.len());
    for m in &t.mips {
        let rgba = decode_level(t.format, m.width, m.height, t.level(m)?)?;
        mips.push(Mip {
            offset: data.len(),
            len: rgba.len(),
            ..*m
        });
        data.extend_from_slice(&rgba);
    }
    // the hash stays: decoded, two textures of one hash and size are still one picture for the renderer
    Some(TextureData {
        format: Format::Rgba,
        data,
        mips,
        hash: t.hash,
    })
}

/// One mip level's blocks as RGBA.
pub fn decode_level(format: Format, width: u32, height: u32, blocks: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = (usize::try_from(width).ok()?, usize::try_from(height).ok()?);
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let size = format.block_bytes();
    if format == Format::Rgba || blocks.len() != bw.checked_mul(bh)?.checked_mul(size)? {
        return None;
    }
    let mut out = vec![0u8; w.checked_mul(h)?.checked_mul(4)?];
    for (k, block) in blocks.chunks_exact(size).enumerate() {
        let px = match format {
            Format::Bc1 => decode_bc1(block.first_chunk::<8>()?),
            _ => decode_bc7(block.first_chunk::<16>()?),
        };
        let (bx, by) = (k.checked_rem(bw)?, k.checked_div(bw)?);
        for (i, p) in px.iter().enumerate() {
            let (x, y) = (
                bx.checked_mul(4)?.checked_add(i % 4)?,
                by.checked_mul(4)?.checked_add(i / 4)?,
            );
            if x < w && y < h {
                let at = y.checked_mul(w)?.checked_add(x)?.checked_mul(4)?;
                out.get_mut(at..at.checked_add(4)?)?.copy_from_slice(p);
            }
        }
    }
    Some(out)
}

/// The colours a BC1 block's two endpoints give (four-colour mode): the endpoints and the two thirds between.
pub fn bc1_palette(c0: u16, c1: u16) -> [[u8; 4]; 4] {
    let rgb = |c: u16| -> [u32; 3] {
        let (r, g, b) = (
            u32::from(c >> 11),
            u32::from((c >> 5) & 63),
            u32::from(c & 31),
        );
        [
            (r << 3) | (r >> 2),
            (g << 2) | (g >> 4),
            (b << 3) | (b >> 2),
        ]
    };
    let (a, b) = (rgb(c0), rgb(c1));
    let mix = |x: [u32; 3], y: [u32; 3]| -> [u8; 4] {
        let c = |i: usize| -> u8 {
            let (x, y) = (
                x.get(i).copied().unwrap_or(0),
                y.get(i).copied().unwrap_or(0),
            );
            u8::try_from(x.wrapping_mul(2).wrapping_add(y).wrapping_add(1) / 3).unwrap_or(u8::MAX)
        };
        [c(0), c(1), c(2), 255]
    };
    let own = |x: [u32; 3]| -> [u8; 4] {
        let c = |i: usize| u8::try_from(x.get(i).copied().unwrap_or(0)).unwrap_or(u8::MAX);
        [c(0), c(1), c(2), 255]
    };
    [own(a), own(b), mix(a, b), mix(b, a)]
}

fn decode_bc1(b: &[u8; 8]) -> [[u8; 4]; 16] {
    let palette = bc1_palette(
        u16::from_le_bytes([b[0], b[1]]),
        u16::from_le_bytes([b[2], b[3]]),
    );
    let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    let mut out = [[0u8; 4]; 16];
    for (i, p) in out.iter_mut().enumerate() {
        let index = (bits >> (u32::try_from(i).unwrap_or(0).wrapping_mul(2))) & 3;
        *p = palette
            .get(usize::try_from(index).unwrap_or(0))
            .copied()
            .unwrap_or_default();
    }
    out
}

/// BC7's weights of its 4-bit indices (of 64).
pub const BC7_WEIGHTS: [u32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

/// A value between two endpoints as BC7 interpolates it.
pub fn bc7_mix(a: u8, b: u8, weight: u32) -> u8 {
    let w = weight.min(64);
    let v = u32::from(a)
        .wrapping_mul(64u32.wrapping_sub(w))
        .wrapping_add(u32::from(b).wrapping_mul(w))
        .wrapping_add(32)
        >> 6;
    u8::try_from(v).unwrap_or(u8::MAX)
}

/// A BC7 block of mode 6 (any other mode decodes as transparent black; `read` lets none through).
fn decode_bc7(b: &[u8; 16]) -> [[u8; 4]; 16] {
    let mut out = [[0u8; 4]; 16];
    let bits = u128::from_le_bytes(*b);
    if bits & 0x7f != 0x40 {
        return out;
    }
    let field = |at: u32, n: u32| -> u32 {
        u32::try_from((bits >> at) & ((1u128 << n).wrapping_sub(1))).unwrap_or(0)
    };
    // two endpoints of 7 bits a channel (r0 r1 g0 g1 b0 b1 a0 a1), then one more low bit for each endpoint
    let (p0, p1) = (field(63, 1), field(64, 1));
    let mut ends = [[0u8; 4]; 2];
    for c in 0..4u32 {
        let at = 7u32.wrapping_add(c.wrapping_mul(14));
        let e = |v: u32, p: u32| u8::try_from((v << 1) | p).unwrap_or(u8::MAX);
        if let Some(x) = ends[0].get_mut(usize::try_from(c).unwrap_or(0)) {
            *x = e(field(at, 7), p0);
        }
        if let Some(x) = ends[1].get_mut(usize::try_from(c).unwrap_or(0)) {
            *x = e(field(at.wrapping_add(7), 7), p1);
        }
    }
    // the indices: 3 bits for the first pixel (its top bit is always 0), 4 for each of the others
    let mut at = 65u32;
    for (i, p) in out.iter_mut().enumerate() {
        let n = if i == 0 { 3 } else { 4 };
        let index = field(at, n);
        at = at.wrapping_add(n);
        let w = BC7_WEIGHTS
            .get(usize::try_from(index).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        for (c, v) in p.iter_mut().enumerate() {
            *v = bc7_mix(
                ends[0].get(c).copied().unwrap_or(0),
                ends[1].get(c).copied().unwrap_or(0),
                w,
            );
        }
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    /// A whole chain of valid blocks for an 8 x 4 texture: 2 blocks, then 1, 1, 1.
    fn levels(format: Format) -> Vec<Vec<u8>> {
        let block: Vec<u8> = match format {
            // endpoints 0xffff > 0x0000, indices 0 1 2 3 ...
            Format::Bc1 => vec![0xff, 0xff, 0, 0, 0xe4, 0xe4, 0xe4, 0xe4],
            _ => {
                let mut b = [0u8; 16];
                b[0] = 0x40;
                b.to_vec()
            }
        };
        [2usize, 1, 1, 1].iter().map(|n| block.repeat(*n)).collect()
    }

    #[test]
    fn a_file_reads_back_and_drops_its_top_levels() {
        for format in [Format::Bc1, Format::Bc7] {
            let file = write(format, 8, 4, (123, 456), &levels(format)).unwrap();
            let h = read(&file).unwrap();
            assert_eq!(
                (h.width, h.height, h.source_len, h.source_hash),
                (8, 4, 123, 456)
            );
            assert_eq!(h.mips.len(), 4);
            // 8 x 4 halves to 4 x 2, which is not whole blocks: nothing can be dropped
            assert_eq!(h.droppable(), 0);
            let t = texture(file.clone(), 3).unwrap();
            assert_eq!(t.size(), (8, 4));
            assert_eq!(t.bytes(), 5 * format.block_bytes());
            let rgba = decoded(&t).unwrap();
            assert_eq!(rgba.mips[0].len, 8 * 4 * 4);
            assert_eq!(rgba.mips[3].len, 4);
        }
        assert!(write(Format::Bc7, 8, 8, (0, 0), &[]).is_err());
    }

    #[test]
    fn a_damaged_file_is_refused() {
        let file = write(Format::Bc1, 8, 4, (0, 0), &levels(Format::Bc1)).unwrap();
        // cut, grown, a flipped data byte, a three-colour block, a side that is not whole blocks, a short chain
        assert!(read(&file[..file.len() - 1]).is_err());
        let mut grown = file.clone();
        grown.push(0);
        assert!(read(&grown).is_err());
        let mut flipped = file.clone();
        *flipped.last_mut().unwrap() ^= 1;
        assert!(read(&flipped).is_err());
        let mut l = levels(Format::Bc1);
        l[0][..4].copy_from_slice(&[0, 0, 0xff, 0xff]);
        assert!(write(Format::Bc1, 8, 4, (0, 0), &l).is_err());
        let mut odd = file.clone();
        odd[8] = 6;
        assert!(header(&odd).is_err());
        let mut short = file.clone();
        short[7] = 3;
        assert!(header(&short).is_err());
        let mut huge = file;
        huge[8..12].copy_from_slice(&8192u32.to_le_bytes());
        assert!(header(&huge).is_err());
        // a BC7 block of another mode
        let mut l = levels(Format::Bc7);
        l[0][0] = 0x01;
        assert!(write(Format::Bc7, 8, 4, (0, 0), &l).is_err());
    }

    #[test]
    fn the_blocks_decode_as_the_formats_say() {
        // BC1: white and black endpoints, the indices 0 1 2 3 across each row
        let px = decode_bc1(&[0xff, 0xff, 0, 0, 0xe4, 0xe4, 0xe4, 0xe4]);
        assert_eq!(px[0], [255, 255, 255, 255]);
        assert_eq!(px[1], [0, 0, 0, 255]);
        assert_eq!(px[2], [170, 170, 170, 255]);
        assert_eq!(px[3], [85, 85, 85, 255]);
        // BC7 mode 6: endpoints 0 and 255 in every channel (7 bits 0 with p 0; 7 bits 127 with p 1), indices 0..15
        let mut bits: u128 = 0x40;
        for c in 0..4 {
            bits |= 127u128 << (7 + c * 14 + 7);
        }
        bits |= 1u128 << 64;
        let mut at = 65;
        for i in 0..16u128 {
            let n = if i == 0 { 3 } else { 4 };
            bits |= i << at;
            at += n;
        }
        let px = decode_bc7(&bits.to_le_bytes());
        assert_eq!(px[0], [0; 4]);
        assert_eq!(px[15], [255; 4]);
        assert_eq!(px[1], [bc7_mix(0, 255, 4); 4]);
        assert_eq!(px[8][0], 135);
    }

    #[test]
    fn mips_are_the_box_of_four() {
        let rgba: Vec<u8> = [
            [0u8, 0, 0, 255],
            [100, 0, 0, 255],
            [0, 200, 0, 255],
            [0, 0, 40, 255],
        ]
        .concat();
        let t = TextureData::from_rgba(2, 2, rgba).unwrap();
        assert_eq!(t.mips.len(), 2);
        assert_eq!(t.level(&t.mips[1]).unwrap(), &[25, 50, 10, 255]);
        // a 4 x 1 strip: 2 x 1, then 1 x 1
        let strip = TextureData::from_rgba(4, 1, vec![8; 16]).unwrap();
        assert_eq!(
            strip
                .mips
                .iter()
                .map(|m| (m.width, m.height))
                .collect::<Vec<_>>(),
            [(4, 1), (2, 1), (1, 1)]
        );
        assert!(strip.data.iter().all(|&v| v == 8));
        assert!(TextureData::from_rgba(4, 4, vec![0; 3]).is_none());
    }
}
