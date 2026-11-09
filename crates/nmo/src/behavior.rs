// Behaviour graphs as Virtools saves them: a script is a CKBehavior (class 8) whose state chunk lists its
// sub-behaviours, links (class 6), parameter operations (class 4), input / output / local parameters (classes 2,
// 3, 45) and its own inputs and outputs (class 9). Each decoder reads one object's chunk and returns references
// as indices into the file's object table (`document::reference`), to be resolved by the caller. Everything is
// bounds-checked; a malformed object is an error for that object only. Layout: docs/ck-runtime.md.

use crate::chunk::Chunk;
use crate::document::reference;
use crate::error::{Error, Part, Result};

pub mod class {
    pub const PARAMETER_IN: u32 = 2;
    pub const PARAMETER_OUT: u32 = 3;
    pub const PARAMETER_OPERATION: u32 = 4;
    pub const BEHAVIOR_LINK: u32 = 6;
    pub const BEHAVIOR: u32 = 8;
    pub const BEHAVIOR_IO: u32 = 9;
    pub const PARAMETER_LOCAL: u32 = 45;
}

/// A Virtools GUID (two 32-bit words), naming a building block prototype, a parameter type or an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Guid(pub u32, pub u32);

impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}:{:08x}", self.0, self.1)
    }
}

/// Behaviour flag: the behaviour is a script (a top-level graph attached to an object).
pub const FLAG_SCRIPT: u32 = 0x2;
/// Behaviour flag: a building block (its prototype's function runs it), not a graph.
pub const FLAG_BUILDING_BLOCK: u32 = 0x8;
const FLAG_HAS_PROTO: u32 = 0x8000;
const FLAG_HAS_PRIORITY: u32 = 0x4;
const FLAG_HAS_CLASS: u32 = 0x10;
const FLAG_HAS_TARGET: u32 = 0x4_0000;

const MAX_LIST: usize = 65_536;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Behavior {
    pub flags: u32,
    /// The building block's prototype and its version; a graph has none.
    pub proto: Option<(Guid, u32)>,
    pub priority: i32,
    /// The parameter (a local) holding the block's target object, when the block is targetable and has one.
    pub target: Option<usize>,
    pub sub_behaviors: Vec<usize>,
    pub links: Vec<usize>,
    pub operations: Vec<usize>,
    pub inputs_params: Vec<usize>,
    pub output_params: Vec<usize>,
    pub local_params: Vec<usize>,
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
}

fn missing(what: &'static str) -> Error {
    Error::Malformed {
        part: Part::Data,
        what,
    }
}

fn list(chunk: &Chunk, w: &mut crate::chunk::Words<'_>) -> Result<Vec<usize>> {
    let n = w.u32()? as usize;
    if n > MAX_LIST {
        return Err(missing("behaviour list too long"));
    }
    let raw = w.take(n)?;
    // A null reference in a list is dropped; the lists name objects, never holes.
    Ok(raw.iter().filter_map(|&r| reference(chunk, r)).collect())
}

/// A behaviour (class 8): `CKBehavior::Save`'s layout under identifier 0x20.
pub fn behavior(chunk: &Chunk) -> Result<Behavior> {
    let mut w = chunk.section(0x20).ok_or(missing("behaviour state"))?;
    let flags = w.u32()?;
    let mut b = Behavior {
        flags,
        ..Behavior::default()
    };
    if flags & FLAG_HAS_PROTO != 0 {
        let guid = Guid(w.u32()?, w.u32()?);
        b.proto = Some((guid, w.u32()?));
    }
    if flags & FLAG_HAS_PRIORITY != 0 {
        b.priority = w.u32()? as i32;
    }
    if flags & FLAG_HAS_CLASS != 0 {
        w.u32()?;
    }
    if flags & FLAG_HAS_TARGET != 0 {
        b.target = reference(chunk, w.u32()?);
    }
    let mask = w.u32()?;
    // The arrays, in the order they are saved.
    const ARRAYS: [u32; 8] = [
        0x100, 0x8_0000, 0x4000, 0x200, 0x400, 0x2_0000, 0x800, 0x1000,
    ];
    let mut lists: [Vec<usize>; 8] = Default::default();
    for (bit, out) in ARRAYS.iter().zip(lists.iter_mut()) {
        if mask & bit != 0 {
            *out = list(chunk, &mut w)?;
        }
    }
    let [sub, links, ops, pin, pout, local, inputs, outputs] = lists;
    b.sub_behaviors = sub;
    b.links = links;
    b.operations = ops;
    b.inputs_params = pin;
    b.output_params = pout;
    b.local_params = local;
    b.inputs = inputs;
    b.outputs = outputs;
    Ok(b)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    /// Frames the activation waits once the source fires (the saved initial delay).
    pub delay: u16,
    pub source: Option<usize>,
    pub destination: Option<usize>,
}

/// A behaviour link (class 6): delays (low word current, high word initial), source IO, destination IO.
pub fn link(chunk: &Chunk) -> Result<Link> {
    let mut w = chunk.section(0x20).ok_or(missing("link state"))?;
    let delays = w.u32()?;
    let source = reference(chunk, w.u32()?);
    let destination = reference(chunk, w.u32()?);
    Ok(Link {
        delay: (delays >> 16) as u16,
        source,
        destination,
    })
}

/// A behaviour IO (class 9): 1 input, 2 output; 0 when unsaved.
pub fn io_flags(chunk: &Chunk) -> u32 {
    chunk
        .section(0x8)
        .and_then(|mut w| w.u32().ok())
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterIn {
    pub guid: Guid,
    /// The parameter read: a parameter's own value (direct) or another input's source (shared).
    pub source: Option<usize>,
    pub shared: bool,
}

/// An input parameter (class 2): its type and where it reads from.
pub fn parameter_in(chunk: &Chunk) -> Result<ParameterIn> {
    for (ident, shared) in [
        (0x1000u32, false),
        (0x2000, true),
        (0x800, false),
        (0x4000, true),
    ] {
        if let Some(mut w) = chunk.section(ident) {
            let guid = Guid(w.u32()?, w.u32()?);
            let source = if w.remaining() > 0 {
                reference(chunk, w.u32()?)
            } else {
                None
            };
            return Ok(ParameterIn {
                guid,
                source,
                shared,
            });
        }
    }
    Err(missing("input parameter state"))
}

/// A saved parameter value, as the type's loader would see it.
#[derive(Debug, Clone, PartialEq)]
pub enum Saved {
    /// A type with its own sub-chunk (not decoded).
    SubChunk,
    /// The raw bytes of a plain value (int, float, bool, string, vector, matrix, colour...).
    Bytes(Vec<u8>),
    /// A message or attribute type, stored by name.
    Name(Vec<u8>),
    /// A parameter-type value: the GUID of a type.
    Type(Guid),
    Object(Option<usize>),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub guid: Guid,
    pub value: Saved,
    /// Output parameters only: the parameters this one is copied to when it changes.
    pub destinations: Vec<usize>,
}

const MAX_VALUE: usize = 1 << 20;
const PG_MESSAGE_OLD: Guid = Guid(0x2136_61cc, 0x7d1a_2d54);
const PG_ATTRIBUTE_OLD: Guid = Guid(0x13b9_7e4c, 0x982e_8b4f);
const PG_PARAMTYPE: Guid = Guid(0x3451_7df5, 0x045e_4965);

/// Legacy type GUIDs the loader remaps before the type lookup (CK2.dll FUN_2400867c).
pub fn remap(guid: Guid) -> Guid {
    match guid {
        PG_MESSAGE_OLD => Guid(0x0388_1e12, 0x5ba3_4e2b),
        PG_ATTRIBUTE_OLD => Guid(0x3ea3_4ee9, 0x09fa_5366),
        Guid(0x7165_3557, 0x2d1b_2e97) => Guid(0x30ec_20ab, 0x6df6_517d),
        Guid(0x4a4d_4867, 0x3c28_773f) => Guid(0x54b4_422b, 0x730f_0f4f),
        g => g,
    }
}

/// An output or local parameter (classes 3 and 45): type, value and, for outputs, destinations.
pub fn parameter(chunk: &Chunk) -> Result<Parameter> {
    let mut w = chunk.section(0x40).ok_or(missing("parameter state"))?;
    let stored = Guid(w.u32()?, w.u32()?);
    let guid = remap(stored);
    let mode = w.u32()?;
    let value = match mode {
        0 => Saved::SubChunk,
        1 => {
            if stored == PG_PARAMTYPE {
                Saved::Type(Guid(w.u32()?, w.u32()?))
            } else {
                let size = w.u32()? as usize;
                if size > MAX_VALUE {
                    return Err(missing("parameter value too large"));
                }
                let words = w.take(size.div_ceil(4))?;
                let bytes: Vec<u8> = words
                    .iter()
                    .flat_map(|x| x.to_le_bytes())
                    .take(size)
                    .collect();
                if stored == PG_MESSAGE_OLD || stored == PG_ATTRIBUTE_OLD {
                    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                    Saved::Name(bytes.get(..end).unwrap_or_default().to_vec())
                } else {
                    Saved::Bytes(bytes)
                }
            }
        }
        2 => Saved::Object(reference(chunk, w.u32()?)),
        3 => Saved::None,
        _ => {
            w.u32()?;
            Saved::Bytes(w.u32()?.to_le_bytes().to_vec())
        }
    };
    let destinations = match chunk.section(0x20) {
        Some(mut d) => list(chunk, &mut d).unwrap_or_default(),
        None => Vec::new(),
    };
    Ok(Parameter {
        guid,
        value,
        destinations,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub guid: Guid,
    pub inputs: [Option<usize>; 2],
    pub output: Option<usize>,
}

/// A parameter operation (class 4): operation GUID, then its two inputs and output.
pub fn operation(chunk: &Chunk) -> Result<Operation> {
    let mut w = chunk.section(0x400).ok_or(missing("operation state"))?;
    let guid = Guid(w.u32()?, w.u32()?);
    let n = w.u32()?;
    if n != 3 {
        return Err(missing("operation argument count"));
    }
    let a = reference(chunk, w.u32()?);
    let b = reference(chunk, w.u32()?);
    let output = reference(chunk, w.u32()?);
    Ok(Operation {
        guid,
        inputs: [a, b],
        output,
    })
}

/// The scripts attached to a behavioural object (any class deriving CKBeObject): identifier 0x800.
pub fn scripts(chunk: &Chunk) -> Vec<usize> {
    match chunk.section(0x800) {
        Some(mut w) => list(chunk, &mut w).unwrap_or_default(),
        None => Vec::new(),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::chunk::OPTION_FILE;

    fn chunk(sections: &[(u32, Vec<u32>)]) -> Chunk {
        let mut words = Vec::new();
        for (k, (id, data)) in sections.iter().enumerate() {
            let next = words.len() + 2 + data.len();
            words.push(*id);
            words.push(if k + 1 == sections.len() {
                0
            } else {
                next as u32
            });
            words.extend(data);
        }
        Chunk {
            data_version: 7,
            class_id: 0,
            chunk_version: 7,
            options: OPTION_FILE,
            words,
        }
    }

    #[test]
    fn a_building_block_with_a_target_and_lists_is_read() {
        let flags = FLAG_BUILDING_BLOCK | FLAG_HAS_PROTO | FLAG_HAS_TARGET | FLAG_HAS_PRIORITY;
        let words = vec![
            flags,
            0x33b9_9f51,
            0x07d9_5c45,
            1,                              // proto, version
            u32::MAX,                       // priority -1
            7,                              // target
            0x200 | 0x400 | 0x800 | 0x1000, // mask: pin, pout, in, out
            2,
            10,
            11, // pins
            1,
            12, // pouts
            1,
            20, // inputs
            2,
            21,
            u32::MAX, // outputs: a null one is dropped
        ];
        let b = behavior(&chunk(&[(0x20, words)])).unwrap();
        assert_eq!(b.proto, Some((Guid(0x33b9_9f51, 0x07d9_5c45), 1)));
        assert_eq!(b.priority, -1);
        assert_eq!(b.target, Some(7));
        assert_eq!(b.inputs_params, vec![10, 11]);
        assert_eq!(b.output_params, vec![12]);
        assert_eq!(b.inputs, vec![20]);
        assert_eq!(b.outputs, vec![21]);
        assert!(b.sub_behaviors.is_empty() && b.links.is_empty());
    }

    #[test]
    fn truncated_lists_are_refused() {
        let words = vec![FLAG_BUILDING_BLOCK, 0x100, 5, 1, 2];
        assert!(behavior(&chunk(&[(0x20, words)])).is_err());
        assert!(
            behavior(&chunk(&[(0x40, vec![0])])).is_err(),
            "no state at all"
        );
    }

    #[test]
    fn links_parameters_and_operations_are_read() {
        let l = link(&chunk(&[(0x20, vec![12 << 16 | 12, 3, 4])])).unwrap();
        assert_eq!(
            l,
            Link {
                delay: 12,
                source: Some(3),
                destination: Some(4)
            }
        );

        let p = parameter(&chunk(&[
            (0x40, vec![0x5a57_16fd, 0x44e2_76d7, 1, 4, 42]),
            (0x20, vec![1, 9]),
        ]))
        .unwrap();
        assert_eq!(p.value, Saved::Bytes(42u32.to_le_bytes().to_vec()));
        assert_eq!(p.destinations, vec![9]);
        let s = parameter(&chunk(&[(
            0x40,
            vec![
                0x2136_61cc,
                0x7d1a_2d54,
                1,
                3,
                u32::from_le_bytes(*b"Hi\0\0"),
            ],
        )]))
        .unwrap();
        assert_eq!(
            s.guid,
            Guid(0x0388_1e12, 0x5ba3_4e2b),
            "legacy message type remapped"
        );
        assert_eq!(s.value, Saved::Name(b"Hi".to_vec()));
        let o = parameter(&chunk(&[(0x40, vec![1, 2, 2, 17])])).unwrap();
        assert_eq!(o.value, Saved::Object(Some(17)));
        assert!(
            parameter(&chunk(&[(0x40, vec![1, 2, 1, 1 << 30])])).is_err(),
            "oversized value"
        );

        let i = parameter_in(&chunk(&[(0x2000, vec![1, 2, 5])])).unwrap();
        assert!(i.shared && i.source == Some(5));

        let op = operation(&chunk(&[(0x400, vec![1, 2, 3, 5, u32::MAX, 6])])).unwrap();
        assert_eq!(op.inputs, [Some(5), None]);
        assert_eq!(op.output, Some(6));
        assert!(operation(&chunk(&[(0x400, vec![1, 2, 2, 5, 6])])).is_err());

        assert_eq!(scripts(&chunk(&[(0x800, vec![2, 30, 31])])), vec![30, 31]);
        assert_eq!(io_flags(&chunk(&[(0x8, vec![2])])), 2);
    }
}
