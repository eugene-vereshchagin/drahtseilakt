// A cursor over untrusted bytes. Every read is bounds-checked and fails with the offset it stopped at; nothing
// here can panic, whatever the input.

use crate::error::{Error, Part, Result};

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    part: Part,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], part: Part) -> Self {
        Reader { data, pos: 0, part }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let truncated = Error::Truncated {
            part: self.part,
            offset: self.pos,
            wanted: n,
        };
        let end = self.pos.checked_add(n).ok_or(truncated.clone())?;
        let slice = self.data.get(self.pos..end).ok_or(truncated)?;
        self.pos = end;
        Ok(slice)
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.bytes(N)?);
        Ok(out)
    }

    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }
}
