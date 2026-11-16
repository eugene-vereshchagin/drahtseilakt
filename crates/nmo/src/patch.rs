// Rewrites the pixels of pictures saved inside a Virtools file, in place, and nothing else: the one change the
// shipped community maps need (a map that carries a copy of one of the original game's textures gets our own
// picture of the same sides instead; THIRD-PARTY.md). It is not a writer: the new pixels take exactly the bytes the
// old ones had, so every object, every offset and the object table stay as they were; only the data part is packed
// again, its packed size and the checksum in the header follow. The input is as untrusted as any map (CLAUDE.md):
// it is read by the same reader first, every place written is found the way the reader finds it and bounds-checked,
// and the result is read back and compared before it is returned, so a file this cannot patch exactly is refused.

use crate::bytes::Reader;
use crate::checksum::Adler32;
use crate::chunk::Chunk;
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

/// The file with the given pictures put into its textures. Refused (the file is left as it was) when the file does
/// not read, an object is not a texture, a slot has no saved picture, the pixels are not exactly the slot's sides, a
/// picture with transparency would go where the saved one has no alpha, or the result does not read back as asked.
pub fn replace_pictures(bytes: &[u8], pictures: &[NewPicture<'_>]) -> Result<Vec<u8>> {
    if pictures.len() > MAX_PICTURES {
        return Err(Error::TooLarge {
            part: Part::Data,
            what: "pictures to replace",
            value: pictures.len() as u64,
            limit: MAX_PICTURES as u64,
        });
    }
    let file = crate::read(bytes)?;
    let doc = crate::Document::parse(&file)?;
    let chunks = object_chunks(&file)?;
    let mut data = file.data.clone();
    for p in pictures {
        let object = doc.get(p.object).ok_or(bad("texture object index"))?;
        if object.class_id != class::TEXTURE {
            return Err(bad("object to patch is not a texture"));
        }
        let chunk = object.chunk.as_ref().ok_or(bad("texture without data"))?;
        let &(start, len) = chunks.get(p.object).ok_or(bad("texture object index"))?;
        let raw = file
            .data
            .get(start..start.saturating_add(len))
            .ok_or(bad("object span"))?;
        // the words the reader parsed are the ones in the file, from byte 8 of the chunk on
        let reparsed = Chunk::parse(raw, Part::Data)?;
        if reparsed.words != chunk.words {
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
        if slot.planes[3].is_none() && p.rgba.chunks_exact(4).any(|px| px.get(3) != Some(&0xFF)) {
            return Err(bad("transparent picture where the saved one has no alpha"));
        }
        // the byte where word `w` of the section starts in the data part
        let at = |w: usize| -> Option<usize> {
            let word = section.start.checked_add(w)?;
            start.checked_add(8)?.checked_add(word.checked_mul(4)?)
        };
        for (channel, plane) in slot.planes.iter().enumerate() {
            let Some(plane) = plane else { continue };
            let base = at(*plane).ok_or(bad("plane offset"))?;
            let end = base.checked_add(pixels).ok_or(bad("plane offset"))?;
            // the plane must lie inside this object's chunk
            if end > start.saturating_add(len) {
                return Err(bad("plane outside its chunk"));
            }
            let target = data
                .get_mut(base..end)
                .ok_or(bad("plane outside the data"))?;
            // planes are blue, green, red, alpha; the file's first row is the picture's bottom one
            let source = [2usize, 1, 0, 3]
                .get(channel)
                .copied()
                .ok_or(bad("channel"))?;
            for (i, byte) in target.iter_mut().enumerate() {
                let (row, col) = (
                    i.checked_div(slot.width).unwrap_or(0),
                    i.checked_rem(slot.width).unwrap_or(0),
                );
                let top_row = slot.height.saturating_sub(1).saturating_sub(row);
                let px = top_row.saturating_mul(slot.width).saturating_add(col);
                let k = px.saturating_mul(4).saturating_add(source);
                *byte = p.rgba.get(k).copied().ok_or(bad("new picture"))?;
            }
        }
    }
    let out = assemble(bytes, &file, &data)?;
    check(&out, &file, pictures)?;
    Ok(out)
}

/// The file again around a new data part: the fixed header with the data's packed size and the checksum made
/// anew, header 1 and the included files as they were stored.
fn assemble(bytes: &[u8], file: &crate::File, data: &[u8]) -> Result<Vec<u8>> {
    let h = &file.header;
    let mut r = Reader::new(bytes, Part::Header);
    let head = r.bytes(HEADER_LEN)?;
    let header1_raw = r.bytes(h.header1_packed as usize)?;
    let _old_data = r.bytes(h.data_packed as usize)?;
    let tail = r.bytes(r.remaining())?;
    let packed = if h.data_packed == h.data_unpacked {
        data.to_vec()
    } else {
        let z = miniz_oxide::deflate::compress_to_vec_zlib(data, 6);
        // a packed part as long as the unpacked one would be read as stored plain: store it plain then
        if z.len() == data.len() {
            data.to_vec()
        } else {
            z
        }
    };
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
    let mut out = head;
    out.extend_from_slice(header1_raw);
    out.extend_from_slice(&packed);
    out.extend_from_slice(tail);
    Ok(out)
}

/// The patched file reads back with a good checksum, the same objects and object table, and every patched slot
/// holding exactly the new picture.
fn check(out: &[u8], before: &crate::File, pictures: &[NewPicture<'_>]) -> Result<()> {
    let after = crate::read(out)?;
    if !after.checksum_matches()
        || after.header1 != before.header1
        || after.data.len() != before.data.len()
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
