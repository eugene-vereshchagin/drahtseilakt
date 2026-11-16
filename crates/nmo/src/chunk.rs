// State chunks: how Virtools saves each object. A chunk is a version word, a count of 32-bit words, the words,
// then optional lists (object ids, sub-chunks, managers) selected by option bits. Inside the words, fields are
// grouped under identifiers that form a linked list: [identifier, index of the next identifier, fields...].

use crate::error::{Error, Part, Result};

/// Option bit: object references are indices into the file's object table rather than runtime ids.
pub const OPTION_FILE: u8 = 8;
const OPTION_IDS: u8 = 1;
const OPTION_MANAGERS: u8 = 2;
const OPTION_SUBCHUNKS: u8 = 4;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub data_version: u8,
    pub class_id: u8,
    pub chunk_version: u8,
    pub options: u8,
    pub words: Vec<u32>,
}

impl Chunk {
    pub fn parse(raw: &[u8], part: Part) -> Result<Chunk> {
        let mut r = crate::bytes::Reader::new(raw, part);
        let version = r.u32()?;
        let count = r.u32()? as usize;
        // The count is bounded by the bytes present: refuse before allocating.
        if count > r.remaining() / 4 {
            return Err(Error::Truncated {
                part,
                offset: 8,
                wanted: count.saturating_mul(4),
            });
        }
        let words = (0..count).map(|_| r.u32()).collect::<Result<Vec<u32>>>()?;
        let options = (version >> 24) as u8;
        // The trailing lists are not needed to read scene objects, but they must be well formed.
        for bit in [OPTION_IDS, OPTION_SUBCHUNKS, OPTION_MANAGERS] {
            if options & bit != 0 {
                let n = r.u32()? as usize;
                r.bytes(n.checked_mul(4).ok_or(Error::Malformed {
                    part,
                    what: "list length",
                })?)?;
            }
        }
        Ok(Chunk {
            data_version: version as u8,
            class_id: (version >> 8) as u8,
            chunk_version: (version >> 16) as u8,
            options,
            words,
        })
    }

    /// The fields saved under an identifier, if present.
    pub fn section(&self, identifier: u32) -> Option<Words<'_>> {
        let range = self.section_range(identifier)?;
        self.words.get(range).map(|w| Words { words: w, pos: 0 })
    }

    /// Where the fields saved under an identifier lie in `words` (the patcher rewrites them in place).
    pub fn section_range(&self, identifier: u32) -> Option<std::ops::Range<usize>> {
        let mut i = 0usize;
        // Each step moves strictly forward, so a malicious list cannot loop.
        while let (Some(&id), Some(&next)) = (self.words.get(i), self.words.get(i.wrapping_add(1)))
        {
            let next = next as usize;
            let end = if next == 0 { self.words.len() } else { next };
            if id == identifier {
                let start = i.saturating_add(2);
                return (start <= end && end <= self.words.len()).then_some(start..end);
            }
            if next == 0 || next <= i {
                return None;
            }
            i = next;
        }
        None
    }

    pub fn references_are_indices(&self) -> bool {
        self.options & OPTION_FILE != 0
    }
}

/// A cursor over the fields of one identifier.
pub struct Words<'a> {
    words: &'a [u32],
    pos: usize,
}

impl<'a> Words<'a> {
    pub fn remaining(&self) -> usize {
        self.words.len().saturating_sub(self.pos)
    }

    pub fn u32(&mut self) -> Result<u32> {
        let w = self.words.get(self.pos).copied().ok_or(Error::Malformed {
            part: Part::Data,
            what: "field",
        })?;
        self.pos = self.pos.saturating_add(1);
        Ok(w)
    }

    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u32]> {
        let end = self.pos.checked_add(n).ok_or(Error::Malformed {
            part: Part::Data,
            what: "length",
        })?;
        let s = self.words.get(self.pos..end).ok_or(Error::Malformed {
            part: Part::Data,
            what: "array",
        })?;
        self.pos = end;
        Ok(s)
    }

    /// A sub-chunk stored inline (CKStateChunk::ReadSubChunk, parent chunk version 4 and later): its size in
    /// words, then class id, version, word count, file flag, id count, sub-chunk count, manager count (parent
    /// version above 4), then the words. Only the words are kept (the lists behind them are skipped).
    pub fn sub_chunk(&mut self, parent_chunk_version: u8) -> Result<Chunk> {
        let bad = |what| Error::Malformed {
            part: Part::Data,
            what,
        };
        let size = self.u32()? as usize;
        if size == 0 || size > self.remaining() || parent_chunk_version < 4 {
            return Err(bad("sub-chunk size"));
        }
        let start = self.pos;
        let class_id = self.u32()?;
        let version = self.u32()?;
        let count = self.u32()? as usize;
        let file = self.u32()?;
        let ids = self.u32()? as usize;
        let subs = self.u32()? as usize;
        let managers = if parent_chunk_version > 4 {
            self.u32()? as usize
        } else {
            0
        };
        let need = count
            .checked_add(ids)
            .and_then(|n| n.checked_add(subs))
            .and_then(|n| n.checked_add(managers))
            .ok_or(bad("sub-chunk lists"))?;
        if need > self.remaining() {
            return Err(bad("sub-chunk length"));
        }
        let words = self.take(count)?.to_vec();
        let options = (if file == 1 { 8 } else { 0 })
            | (if ids != 0 { 1 } else { 0 })
            | (if subs != 0 { 4 } else { 0 })
            | (if managers != 0 { 2 } else { 0 });
        self.pos = start.saturating_add(size).min(self.words.len());
        Ok(Chunk {
            data_version: version as u8,
            class_id: class_id as u8,
            chunk_version: (version >> 16) as u8,
            options,
            words,
        })
    }

    /// A string: its length in bytes, then the bytes padded to whole words. The trailing NUL and anything
    /// after it (some writers leave garbage in the padding) are dropped.
    pub fn string(&mut self) -> Result<Vec<u8>> {
        let len = self.u32()? as usize;
        let words = self.take(len.div_ceil(4))?;
        let mut bytes: Vec<u8> = words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .take(len)
            .collect();
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        Ok(bytes)
    }
}
