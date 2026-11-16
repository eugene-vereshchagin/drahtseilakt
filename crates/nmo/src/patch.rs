// Rewrites the pixels of pictures saved inside a Virtools file, in place, and nothing else: the one change the
// shipped community maps need (a map that carries a copy of one of the original game's textures gets our own
// picture of the same sides instead; THIRD-PARTY.md). It is not a writer: the new pixels take exactly the bytes the
// old ones had, so every object, every offset and the object table stay as they were; only the data part is packed
// again, its packed size and the checksum in the header follow. The input is as untrusted as any map (CLAUDE.md):
// it is read by the same reader first, every place written is found the way the reader finds it and bounds-checked,
// and the result is read back and compared before it is returned, so a file this cannot patch exactly is refused.

use crate::bytes::Reader;
use crate::checksum::Adler32;
use crate::document::class;
use crate::error::{Error, Part, Result};
use crate::file::limits;

/// A picture to put into a texture: the texture object's index in the file, its slot (0 the first) and the new
/// pixels, RGBA rows top down, of exactly the sides of the slot's saved picture.
#[derive(Debug, Clone, Copy)]
pub struct NewPicture<'a> {
    pub object: usize,
    pub slot: usize,
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}

/// The most pictures one call rewrites.
const MAX_PICTURES: usize = 4_096;

/// The largest unpacked header 1 or data part patched (the biggest shipped map is about 3 MiB unpacked).
const MAX_UNPACKED: u64 = 64 * 1024 * 1024;
/// The file versions whose checksum and object layout the patcher writes (version 8's: the checksum covers the
/// header and both stored parts). Older files sum differently and lay their objects out differently.
const PATCH_VERSIONS: [u32; 2] = [8, 9];
/// Write mode bits: chunks compressed one by one (old files; not patched), the whole data part compressed.
const CHUNK_COMPRESSED_OLD: u32 = 1;
const WHOLE_COMPRESSED: u32 = 8;

const HEADER_LEN: usize = 64;
/// Byte offsets in the fixed header (file.rs reads them in this order).
const AT_CHECKSUM: usize = 8;
const AT_DATA_PACKED: usize = 32;

fn bad(what: &'static str) -> Error {
    Error::Malformed {
        part: Part::Data,
        what,
    }
}

/// Where each object's state chunk lies in the unpacked data part: (start, length), in object order. Walked the
/// way `Document::parse` walks it (the managers' chunks first).
fn object_chunks(file: &crate::File) -> Result<Vec<(usize, usize)>> {
    let mut r = Reader::new(&file.data, Part::Data);
    for _ in 0..file.header.manager_count {
        r.bytes(8)?;
        let size = r.u32()? as usize;
        r.bytes(size)?;
    }
    let count = file.header.object_count as usize;
    let mut out = Vec::with_capacity(count.min(file.data.len() / 4));
    for _ in 0..count {
        let size = r.u32()? as usize;
        let start = file.data.len().saturating_sub(r.remaining());
        r.bytes(size)?;
        out.push((start, size));
    }
    if r.remaining() != 0 {
        return Err(bad("object data (bytes left over)"));
    }
    Ok(out)
}

/// One saved picture of a texture: its sides and where its four planes' bytes start, as word indices into the
/// section's words (blue, green, red, then alpha when the picture has one).
struct SlotPlanes {
    width: usize,
    height: usize,
    planes: [Option<usize>; 4],
}

/// The saved pictures of a texture's section 0x20000, found exactly as `scene::texture` reads them: a picture it
/// would not read ends the list here too.
fn slot_planes(words: &[u32]) -> Vec<SlotPlanes> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let word = |pos: &mut usize| -> Option<u32> {
        let w = words.get(*pos).copied()?;
        *pos = pos.checked_add(1)?;
        Some(w)
    };
    let Some(slots) = word(&mut pos) else {
        return out;
    };
    let mut budget = crate::scene::MAX_SLOT_BYTES;
    for _ in 0..(slots as usize).min(crate::scene::MAX_SLOTS) {
        if words.len().saturating_sub(pos) < 9 {
            break;
        }
        let (Some(bpp), Some(width), Some(height)) =
            (word(&mut pos), word(&mut pos), word(&mut pos))
        else {
            break;
        };
        pos = pos.saturating_add(4); // the channel masks
        let Some(compression) = word(&mut pos) else {
            break;
        };
        let side = 1..=crate::scene::MAX_TEXTURE_SIDE;
        if !(bpp == 32 && compression == 0 && side.contains(&width) && side.contains(&height)) {
            break;
        }
        let pixels = (width as usize).saturating_mul(height as usize);
        if pixels.saturating_mul(4) > budget {
            break;
        }
        let mut planes = [None; 4];
        let mut whole = true;
        for (k, plane) in planes.iter_mut().enumerate() {
            let optional = k == 3;
            if optional && pos >= words.len() {
                break;
            }
            let Some(size) = word(&mut pos) else {
                whole = false;
                break;
            };
            if optional && size == 0 {
                break;
            }
            let end = pos.checked_add((size as usize).div_ceil(4));
            if size as usize != pixels || end.is_none_or(|e| e > words.len()) {
                whole = false;
                break;
            }
            *plane = Some(pos);
            pos = end.unwrap_or(pos);
        }
        if !whole {
            break;
        }
        budget = budget.saturating_sub(pixels.saturating_mul(4));
        out.push(SlotPlanes {
            width: width as usize,
            height: height as usize,
            planes,
        });
    }
    out
}

/// One plane to rewrite: its bytes in the data part, which channel of which new picture goes there, and the sides
/// of the saved picture.
struct PlaneWrite {
    base: usize,
    end: usize,
    channel: usize,
    picture: usize,
    width: usize,
    height: usize,
}

/// The object chunk's words as the reader parsed them are exactly the bytes in the file from byte 8 on, so a word
/// index in `words` names a byte in the data part.
fn words_are_the_bytes(raw: &[u8], words: &[u32]) -> bool {
    let Some(count) = u32::try_from(words.len()).ok() else {
        return false;
    };
    let Some(body) = words
        .len()
        .checked_mul(4)
        .and_then(|n| n.checked_add(8))
        .and_then(|end| raw.get(8..end))
    else {
        return false;
    };
    raw.get(4..8) == Some(&count.to_le_bytes()[..])
        && body
            .as_chunks::<4>()
            .0
            .iter()
            .zip(words)
            .all(|(b, w)| *b == w.to_le_bytes())
}

/// Where the new pictures go, checked against the file before anything is written.
fn plan(file: &crate::File, pictures: &[NewPicture<'_>]) -> Result<Vec<PlaneWrite>> {
    let doc = crate::Document::parse(file)?;
    let chunks = object_chunks(file)?;
    let mut writes = Vec::new();
    for (index, p) in pictures.iter().enumerate() {
        let object = doc.get(p.object).ok_or(bad("texture object index"))?;
        if object.class_id != class::TEXTURE {
            return Err(bad("object to patch is not a texture"));
        }
        let chunk = object.chunk.as_ref().ok_or(bad("texture without data"))?;
        let &(start, len) = chunks.get(p.object).ok_or(bad("texture object index"))?;
        let chunk_end = start.checked_add(len).ok_or(bad("object span"))?;
        let raw = file.data.get(start..chunk_end).ok_or(bad("object span"))?;
        if !words_are_the_bytes(raw, &chunk.words) {
            return Err(bad("texture chunk"));
        }
        let section = chunk
            .section_range(0x0002_0000)
            .ok_or(bad("texture without saved pictures"))?;
        let words = chunk
            .words
            .get(section.clone())
            .ok_or(bad("texture section"))?;
        let slots = slot_planes(words);
        let slot = slots
            .get(p.slot)
            .ok_or(bad("texture slot without a picture"))?;
        let pixels = slot.width.saturating_mul(slot.height);
        if (p.width as usize, p.height as usize) != (slot.width, slot.height)
            || p.rgba.len() != pixels.saturating_mul(4)
        {
            return Err(bad("new picture's size is not the saved picture's"));
        }
        if slot.planes[3].is_none() && p.rgba.as_chunks::<4>().0.iter().any(|px| px[3] != 0xFF) {
            return Err(bad("transparent picture where the saved one has no alpha"));
        }
        // the byte where word `w` of the section starts in the data part
        let at = |w: usize| -> Option<usize> {
            let word = section.start.checked_add(w)?;
            start.checked_add(8)?.checked_add(word.checked_mul(4)?)
        };
        for (k, plane) in slot.planes.iter().enumerate() {
            let Some(plane) = plane else { continue };
            let base = at(*plane).ok_or(bad("plane offset"))?;
            let end = base.checked_add(pixels).ok_or(bad("plane offset"))?;
            // the plane must lie inside this object's chunk
            if end > chunk_end {
                return Err(bad("plane outside its chunk"));
            }
            // planes are blue, green, red, alpha
            let channel = [2usize, 1, 0, 3].get(k).copied().ok_or(bad("channel"))?;
            writes.push(PlaneWrite {
                base,
                end,
                channel,
                picture: index,
                width: slot.width,
                height: slot.height,
            });
        }
    }
    Ok(writes)
}

/// The file with the given pictures put into its textures. Refused (the file is left as it was) when the file is
/// not a version 8 or 9 file saved whole (not chunk by chunk), its checksum does not match, its data part is larger
/// than any map, it does not read, an object is not a texture, a slot has no saved picture, the pixels are not
/// exactly the slot's sides, a picture with transparency would go where the saved one has no alpha, or the result
/// does not read back exactly as intended.
pub fn replace_pictures(bytes: &[u8], pictures: &[NewPicture<'_>]) -> Result<Vec<u8>> {
    if pictures.len() > MAX_PICTURES {
        return Err(Error::TooLarge {
            part: Part::Data,
            what: "pictures to replace",
            value: pictures.len() as u64,
            limit: MAX_PICTURES as u64,
        });
    }
    // what the header says is checked before anything is unpacked
    let header = crate::file::read_header(bytes)?;
    if !PATCH_VERSIONS.contains(&header.file_version) {
        return Err(Error::UnsupportedVersion {
            file_version: header.file_version,
        });
    }
    if header.write_mode & CHUNK_COMPRESSED_OLD != 0 {
        return Err(Error::Malformed {
            part: Part::Header,
            what: "write mode (chunks compressed one by one)",
        });
    }
    for (part, what, value) in [
        (Part::Header1, "unpacked size", header.header1_unpacked),
        (Part::Data, "unpacked size", header.data_unpacked),
    ] {
        if u64::from(value) > MAX_UNPACKED {
            return Err(Error::TooLarge {
                part,
                what,
                value: value.into(),
                limit: MAX_UNPACKED,
            });
        }
    }
    let mut file = crate::read(bytes)?;
    // a stored checksum that is already wrong would come out as a fresh, valid one: refuse instead
    if !file.checksum_matches() {
        return Err(Error::Malformed {
            part: Part::Header,
            what: "checksum",
        });
    }
    let writes = plan(&file, pictures)?;
    let mut data = std::mem::take(&mut file.data);
    for w in &writes {
        let p = pictures.get(w.picture).ok_or(bad("new picture"))?;
        let target = data
            .get_mut(w.base..w.end)
            .ok_or(bad("plane outside the data"))?;
        // the file's first row is the picture's bottom one
        for (i, byte) in target.iter_mut().enumerate() {
            let (row, col) = (
                i.checked_div(w.width).unwrap_or(0),
                i.checked_rem(w.width).unwrap_or(0),
            );
            let top_row = w.height.saturating_sub(1).saturating_sub(row);
            let px = top_row.saturating_mul(w.width).saturating_add(col);
            let k = px.saturating_mul(4).saturating_add(w.channel);
            *byte = p.rgba.get(k).copied().ok_or(bad("new picture"))?;
        }
    }
    let out = assemble(bytes, &file.header, &data)?;
    check(&out, &file, &data, pictures)?;
    Ok(out)
}

/// The data part as it is stored: packed when the file was saved whole-compressed and packing makes it smaller,
/// plain otherwise (a part whose stored size equals its unpacked size is read as plain).
fn stored_data(header: &crate::Header, data: &[u8]) -> Vec<u8> {
    if header.write_mode & WHOLE_COMPRESSED != 0 {
        let z = miniz_oxide::deflate::compress_to_vec_zlib(data, 6);
        if z.len() < data.len() {
            return z;
        }
    }
    data.to_vec()
}

/// The file again around a new data part: the fixed header with the data's packed size and the checksum made
/// anew (the version 8 checksum: header, header 1 and data as stored), header 1 and the included files as they
/// were stored.
fn assemble(bytes: &[u8], h: &crate::Header, data: &[u8]) -> Result<Vec<u8>> {
    let mut r = Reader::new(bytes, Part::Header);
    let head = r.bytes(HEADER_LEN)?;
    let header1_raw = r.bytes(h.header1_packed as usize)?;
    let _old_data = r.bytes(h.data_packed as usize)?;
    let tail = r.bytes(r.remaining())?;
    let packed = stored_data(h, data);
    let packed_len = u32::try_from(packed.len())
        .ok()
        .filter(|&n| u64::from(n) <= limits::MAX_PART)
        .ok_or(Error::TooLarge {
            part: Part::Data,
            what: "packed size",
            value: packed.len() as u64,
            limit: limits::MAX_PART,
        })?;
    let mut head = head.to_vec();
    let put = |head: &mut Vec<u8>, at: usize, v: u32| -> Result<()> {
        head.get_mut(at..at.saturating_add(4))
            .ok_or(bad("header"))?
            .copy_from_slice(&v.to_le_bytes());
        Ok(())
    };
    put(&mut head, AT_DATA_PACKED, packed_len)?;
    put(&mut head, AT_CHECKSUM, 0)?;
    let mut sum = Adler32::virtools();
    sum.update(&head);
    sum.update(header1_raw);
    sum.update(&packed);
    put(&mut head, AT_CHECKSUM, sum.finish())?;
    let total = HEADER_LEN
        .checked_add(header1_raw.len())
        .and_then(|n| n.checked_add(packed.len()))
        .and_then(|n| n.checked_add(tail.len()))
        .ok_or(bad("file size"))?;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&head);
    out.extend_from_slice(header1_raw);
    out.extend_from_slice(&packed);
    out.extend_from_slice(tail);
    Ok(out)
}

/// The patched file reads back with a good checksum, the same header (but the data's packed size and the
/// checksum), header 1 and included files, exactly the intended data part, and every patched slot holding exactly
/// the new picture as the scene reader sees it.
fn check(out: &[u8], before: &crate::File, data: &[u8], pictures: &[NewPicture<'_>]) -> Result<()> {
    let after = crate::read(out)?;
    let same_header = crate::Header {
        checksum: before.header.checksum,
        data_packed: before.header.data_packed,
        ..after.header.clone()
    } == before.header;
    if !after.checksum_matches()
        || !same_header
        || after.header1 != before.header1
        || after.data != data
        || after.included != before.included
    {
        return Err(bad("patched file does not read back"));
    }
    let doc = crate::Document::parse(&after)?;
    for p in pictures {
        let chunk = doc
            .get(p.object)
            .and_then(|o| o.chunk.as_ref())
            .ok_or(bad("patched texture"))?;
        let t = crate::scene::texture(chunk)?;
        let saved = std::iter::once(t.embedded)
            .chain(t.slots.into_iter().map(|s| s.embedded))
            .nth(p.slot)
            .flatten()
            .ok_or(bad("patched slot"))?;
        if (saved.0, saved.1) != (p.width, p.height) || saved.2 != p.rgba {
            return Err(bad("patched picture does not read back"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
mod tests {
    use super::*;

    fn header(write_mode: u32) -> crate::Header {
        crate::Header {
            checksum: 0,
            ck_version: 0x1302_2002,
            file_version: 8,
            write_mode,
            header1_packed: 0,
            data_packed: 0,
            data_unpacked: 0,
            manager_count: 0,
            object_count: 0,
            max_id_saved: 0,
            product_version: 0,
            product_build: 0,
            header1_unpacked: 0,
        }
    }

    /// Bytes zlib cannot shrink.
    fn noise(n: usize) -> Vec<u8> {
        let mut x = 0x9E37_79B9u32;
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x.to_le_bytes()[3]
            })
            .collect()
    }

    #[test]
    fn data_is_packed_only_when_saved_whole_compressed_and_packing_shrinks_it() {
        let flat = vec![7u8; 4_096];
        let random = noise(4_096);
        assert!(miniz_oxide::deflate::compress_to_vec_zlib(&random, 6).len() >= random.len());
        // whole-compressed: flat data packs, noise and a tiny part are stored plain
        let packed = stored_data(&header(WHOLE_COMPRESSED), &flat);
        assert!(packed.len() < flat.len());
        assert_eq!(
            miniz_oxide::inflate::decompress_to_vec_zlib(&packed).ok(),
            Some(flat.clone())
        );
        assert_eq!(stored_data(&header(WHOLE_COMPRESSED), &random), random);
        assert_eq!(stored_data(&header(WHOLE_COMPRESSED), &[1, 2]), vec![1, 2]);
        // not whole-compressed (other bits set): always plain
        assert_eq!(stored_data(&header(0), &flat), flat);
        assert_eq!(stored_data(&header(4 | 2), &flat), flat);
    }
}
