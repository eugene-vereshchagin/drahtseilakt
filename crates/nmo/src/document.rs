// The objects of a file: the object table from header 1, matched with one state chunk per object from the data
// part (after one chunk per manager). Objects are decoded by class on demand; a broken object is an error for
// that object only, so one bad mesh in a community map does not hide the rest of the level.

use crate::bytes::Reader;
use crate::chunk::Chunk;
use crate::error::{Error, Part, Result};
use crate::file::File;

const MAX_NAME: usize = 4_096;

/// Class ids of the objects a level is made of (Virtools CKCID values).
pub mod class {
    pub const LEVEL: u32 = 21;
    pub const DATA_ARRAY: u32 = 52;
    pub const GROUP: u32 = 23;
    pub const MATERIAL: u32 = 30;
    pub const TEXTURE: u32 = 31;
    pub const MESH: u32 = 32;
    pub const ENTITY_3D: u32 = 33;
    pub const CAMERA: u32 = 34;
    pub const TARGET_CAMERA: u32 = 35;
    pub const LIGHT: u32 = 38;
    pub const TARGET_LIGHT: u32 = 39;
    pub const OBJECT_3D: u32 = 41;
    pub const SPRITE_3D: u32 = 37;
    pub const CURVE: u32 = 43;
    pub const OBJECT_ANIMATION: u32 = 15;
    pub const CURVE_POINT: u32 = 36;
    pub const WAVE_SOUND: u32 = 25;
    pub const MIDI_SOUND: u32 = 26;
    /// A 2D frame; 28 is a 2D sprite, 29 a sprite's text (both 2D entities with a picture of their own).
    pub const ENTITY_2D: u32 = 27;
    pub const SPRITE_2D: u32 = 28;
    pub const SPRITE_TEXT: u32 = 29;
}

#[derive(Debug, Clone)]
pub struct Object {
    pub id: u32,
    pub class_id: u32,
    /// As stored: bytes in the writer's code page, not necessarily UTF-8.
    pub name: Vec<u8>,
    pub chunk: Option<Chunk>,
}

impl Object {
    pub fn name_lossy(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub objects: Vec<Object>,
    /// The message manager's names in file index order (a Message parameter saved as an index reads one).
    pub messages: Vec<Vec<u8>>,
}

impl Document {
    pub fn parse(file: &File) -> Result<Document> {
        let count = file.header.object_count as usize;
        let mut table = Reader::new(&file.header1, Part::Header1);
        let mut entries = Vec::with_capacity(count.min(file.header1.len() / 16));
        for _ in 0..count {
            let id = table.u32()?;
            let class_id = table.u32()?;
            let _file_index = table.u32()?;
            let name_len = table.u32()? as usize;
            if name_len > MAX_NAME {
                return Err(Error::TooLarge {
                    part: Part::Header1,
                    what: "object name",
                    value: name_len as u64,
                    limit: MAX_NAME as u64,
                });
            }
            let mut name = table.bytes(name_len)?.to_vec();
            if let Some(nul) = name.iter().position(|&b| b == 0) {
                name.truncate(nul);
            }
            entries.push((id, class_id, name));
        }

        let mut data = Reader::new(&file.data, Part::Data);
        let mut messages = Vec::new();
        for _ in 0..file.header.manager_count {
            let guid = data.bytes(8)?; // manager GUID
            let size = data.u32()? as usize;
            let raw = data.bytes(size)?;
            if guid.get(..4) == Some(&MESSAGE_MANAGER.to_le_bytes()[..]) && size > 0 {
                // a manager chunk that does not parse leaves the table empty: indices then stay unnamed
                if let Ok(chunk) = Chunk::parse(raw, Part::Data) {
                    messages = message_names(&chunk).unwrap_or_default();
                }
            }
        }
        let mut objects = Vec::with_capacity(entries.len());
        for (id, class_id, name) in entries {
            let size = data.u32()? as usize;
            let raw = data.bytes(size)?;
            let chunk = if size == 0 {
                None
            } else {
                Some(Chunk::parse(raw, Part::Data)?)
            };
            objects.push(Object {
                id,
                class_id,
                name,
                chunk,
            });
        }
        if data.remaining() != 0 {
            return Err(Error::Malformed {
                part: Part::Data,
                what: "object data (bytes left over)",
            });
        }
        Ok(Document { objects, messages })
    }

    pub fn get(&self, index: usize) -> Option<&Object> {
        self.objects.get(index)
    }

    pub fn of_class(&self, class_id: u32) -> impl Iterator<Item = (usize, &Object)> {
        self.objects
            .iter()
            .enumerate()
            .filter(move |(_, o)| o.class_id == class_id)
    }
}

/// The message manager's GUID (first word); its chunk's section 0x53 is a count and the names.
const MESSAGE_MANAGER: u32 = 0x466a_0fac;
/// The most message types a file names.
const MAX_MESSAGE_TYPES: usize = 4_096;

fn message_names(chunk: &Chunk) -> Option<Vec<Vec<u8>>> {
    let mut w = chunk.section(0x53)?;
    let n = (w.u32().ok()? as usize).min(MAX_MESSAGE_TYPES);
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(w.string().ok()?);
    }
    Some(out)
}

/// A reference to another object in the same file, resolved to its index; `None` for a null reference.
pub fn reference(chunk: &Chunk, raw: u32) -> Option<usize> {
    if raw == u32::MAX || !chunk.references_are_indices() {
        // Runtime ids appear only in chunks not saved for a file; level objects never use them.
        return None;
    }
    Some(raw as usize)
}
