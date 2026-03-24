use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("not a Mach-O file (bad magic: {magic:#010x})")]
    BadMagic { magic: u32 },

    #[error("unexpected end of data at offset {offset} (need {needed} bytes, have {available})")]
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },

    #[error("invalid UTF-8 in {context}: {source}")]
    InvalidUtf8 {
        context: &'static str,
        source: std::str::Utf8Error,
    },

    #[error("load command size {size} is too small (min {min})")]
    LoadCommandTooSmall { size: u32, min: u32 },

    #[error("segment section count {count} would exceed remaining data")]
    TooManySections { count: u32 },

    #[error("{0}")]
    Other(String),
}
