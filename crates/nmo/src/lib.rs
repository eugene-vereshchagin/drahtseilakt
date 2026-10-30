// Reader for Virtools composition files (.nmo / .cmo), the format of Ballance's levels and community maps.
// Every input is treated as hostile (community maps come from strangers' file shares): reads are
// bounds-checked, sizes are capped before allocating, and a malformed file is an error, never a panic.

mod bytes;
mod checksum;
pub mod chunk;
pub mod document;
pub mod error;
pub mod file;
pub mod scene;

pub use document::{Document, Object, class};
pub use error::{Error, Part, Result};
pub use file::{File, Header, IncludedFile, read};
