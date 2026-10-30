// Errors of the reader. Every failure names where in the file it happened, so a broken community map can be
// reported precisely instead of crashing the game.

use std::fmt;

/// Where in the file a problem was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The fixed 64-byte header at the start of the file.
    Header,
    /// The object table and its companions (header 1), after decompression.
    Header1,
    /// The object and manager data, after decompression.
    Data,
    /// The files embedded after the data.
    Included,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The data ends before a field that must be there.
    Truncated {
        part: Part,
        offset: usize,
        wanted: usize,
    },
    /// The file does not start with the Virtools signature.
    NotVirtools,
    /// A file version this reader does not understand.
    UnsupportedVersion { file_version: u32 },
    /// A size or count beyond what any real file needs; refused before allocating.
    TooLarge {
        part: Part,
        what: &'static str,
        value: u64,
        limit: u64,
    },
    /// A compressed part does not inflate to its declared size.
    Decompress { part: Part },
    /// A structure inside a part does not fit its own declared shape.
    Malformed { part: Part, what: &'static str },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated {
                part,
                offset,
                wanted,
            } => {
                write!(
                    f,
                    "{part:?}: truncated at offset {offset}, {wanted} more bytes needed"
                )
            }
            Error::NotVirtools => write!(f, "not a Virtools file (no \"Nemo Fi\" signature)"),
            Error::UnsupportedVersion { file_version } => {
                write!(f, "unsupported file version {file_version}")
            }
            Error::TooLarge {
                part,
                what,
                value,
                limit,
            } => {
                write!(f, "{part:?}: {what} is {value}, above the limit of {limit}")
            }
            Error::Decompress { part } => write!(f, "{part:?}: compressed data is invalid"),
            Error::Malformed { part, what } => write!(f, "{part:?}: malformed {what}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
