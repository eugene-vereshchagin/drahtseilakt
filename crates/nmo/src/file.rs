// The container of a Virtools composition file: the fixed header, then header 1 (the object table and its
// companions) and the data (objects and managers), each stored either plain or zlib-compressed. This layer
// checks the checksum, sizes and decompression; it does not interpret the two parts yet.

use crate::bytes::Reader;
use crate::checksum::Adler32;
use crate::error::{Error, Part, Result};

const SIGNATURE: [u8; 8] = *b"Nemo Fi\0";
const HEADER_LEN: usize = 64;

/// Caps that no real file comes near; anything above is refused before memory is allocated.
pub mod limits {
    /// Largest decompressed size of either part (the biggest original file is about 3 MiB unpacked).
    pub const MAX_PART: u64 = 256 * 1024 * 1024;
    pub const MAX_OBJECTS: u64 = 1_000_000;
    pub const MAX_MANAGERS: u64 = 1_024;
    pub const MAX_INCLUDED_FILES: usize = 4_096;
    pub const MAX_INCLUDED_NAME: usize = 1_024;
}

/// The fixed 64-byte header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub checksum: u32,
    /// Version of the Virtools engine that wrote the file (0x13022002 for Ballance's Virtools 2.1).
    pub ck_version: u32,
    pub file_version: u32,
    pub write_mode: u32,
    pub header1_packed: u32,
    pub data_packed: u32,
    pub data_unpacked: u32,
    pub manager_count: u32,
    pub object_count: u32,
    pub max_id_saved: u32,
    pub product_version: u32,
    pub product_build: u32,
    pub header1_unpacked: u32,
}

/// A file whose container is valid: header checked, both parts decompressed to their declared sizes.
#[derive(Debug, Clone)]
pub struct File {
    pub header: Header,
    /// The checksum recomputed over the header, header 1 and data. It matches in every file seen (the game and
    /// ~600 community maps), but whether the original game checks it is unknown, so a mismatch is reported,
    /// not refused. Safety comes from the bounds checks and limits, not from a checksum anyone can recompute.
    pub computed_checksum: u32,
    pub header1: Vec<u8>,
    pub data: Vec<u8>,
    /// Files embedded after the data (Virtools 3.5 adds a `vtthumbnail.jpg` preview, for instance).
    pub included: Vec<IncludedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedFile {
    /// The name as stored: bytes in the writer's code page (often GBK in Chinese maps), not necessarily UTF-8.
    pub name: Vec<u8>,
    pub data: Vec<u8>,
}

impl File {
    pub fn checksum_matches(&self) -> bool {
        self.computed_checksum == self.header.checksum
    }
}

/// The oldest and newest file versions whose layout this reader knows (header 1 exists from version 5).
const VERSIONS: std::ops::RangeInclusive<u32> = 5..=9;

pub fn read(bytes: &[u8]) -> Result<File> {
    let header = read_header(bytes)?;

    let mut r = Reader::new(bytes, Part::Header);
    let head = r.bytes(HEADER_LEN)?;
    let header1_raw = r.bytes(header.header1_packed as usize)?;
    let data_raw = r.bytes(header.data_packed as usize)?;
    let computed_checksum = checksum(head, &[header1_raw, data_raw]);
    let included = read_included(r.bytes(r.remaining())?)?;

    let header1 = unpack(Part::Header1, header1_raw, header.header1_unpacked)?;
    let data = unpack(Part::Data, data_raw, header.data_unpacked)?;
    Ok(File {
        header,
        computed_checksum,
        header1,
        data,
        included,
    })
}

fn read_header(bytes: &[u8]) -> Result<Header> {
    let mut r = Reader::new(bytes, Part::Header);
    if r.array::<8>().ok() != Some(SIGNATURE) {
        return Err(Error::NotVirtools);
    }
    let header = Header {
        checksum: r.u32()?,
        ck_version: r.u32()?,
        file_version: r.u32()?,
        // A field that is zero in every file seen; not interpreted.
        write_mode: {
            r.u32()?;
            r.u32()?
        },
        header1_packed: r.u32()?,
        data_packed: r.u32()?,
        data_unpacked: r.u32()?,
        manager_count: r.u32()?,
        object_count: r.u32()?,
        max_id_saved: r.u32()?,
        product_version: r.u32()?,
        product_build: r.u32()?,
        header1_unpacked: r.u32()?,
    };
    if !VERSIONS.contains(&header.file_version) {
        return Err(Error::UnsupportedVersion {
            file_version: header.file_version,
        });
    }
    let checks: [(Part, &'static str, u32, u64); 6] = [
        (
            Part::Header1,
            "packed size",
            header.header1_packed,
            limits::MAX_PART,
        ),
        (
            Part::Header1,
            "unpacked size",
            header.header1_unpacked,
            limits::MAX_PART,
        ),
        (
            Part::Data,
            "packed size",
            header.data_packed,
            limits::MAX_PART,
        ),
        (
            Part::Data,
            "unpacked size",
            header.data_unpacked,
            limits::MAX_PART,
        ),
        (
            Part::Header,
            "object count",
            header.object_count,
            limits::MAX_OBJECTS,
        ),
        (
            Part::Header,
            "manager count",
            header.manager_count,
            limits::MAX_MANAGERS,
        ),
    ];
    for (part, what, value, limit) in checks {
        if u64::from(value) > limit {
            return Err(Error::TooLarge {
                part,
                what,
                value: value.into(),
                limit,
            });
        }
    }
    Ok(header)
}

/// The header with its checksum field zeroed, then the stored (packed) parts; included files are not covered.
fn checksum(head: &[u8], parts: &[&[u8]]) -> u32 {
    let mut zeroed = head.to_vec();
    zeroed.iter_mut().skip(8).take(4).for_each(|b| *b = 0);
    let mut sum = Adler32::virtools();
    sum.update(&zeroed);
    parts.iter().for_each(|p| sum.update(p));
    sum.finish()
}

/// Embedded files: until the end, a name (u32 length + bytes) and contents (u32 size + bytes).
fn read_included(tail: &[u8]) -> Result<Vec<IncludedFile>> {
    let mut r = Reader::new(tail, Part::Included);
    let mut files = Vec::new();
    while r.remaining() != 0 {
        if files.len() >= limits::MAX_INCLUDED_FILES {
            return Err(Error::TooLarge {
                part: Part::Included,
                what: "file count",
                value: files.len() as u64,
                limit: limits::MAX_INCLUDED_FILES as u64,
            });
        }
        let name_len = r.u32()? as usize;
        if name_len > limits::MAX_INCLUDED_NAME {
            return Err(Error::TooLarge {
                part: Part::Included,
                what: "name length",
                value: name_len as u64,
                limit: limits::MAX_INCLUDED_NAME as u64,
            });
        }
        let name = r.bytes(name_len)?.to_vec();
        // The size needs no separate cap: it is bounded by the bytes actually present.
        let size = r.u32()? as usize;
        let data = r.bytes(size)?.to_vec();
        files.push(IncludedFile { name, data });
    }
    Ok(files)
}

/// A part is compressed exactly when its packed and unpacked sizes differ.
fn unpack(part: Part, raw: &[u8], unpacked: u32) -> Result<Vec<u8>> {
    let size = unpacked as usize;
    if raw.len() == size {
        return Ok(raw.to_vec());
    }
    let out = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(raw, size)
        .map_err(|_| Error::Decompress { part })?;
    if out.len() != size {
        return Err(Error::Decompress { part });
    }
    Ok(out)
}
