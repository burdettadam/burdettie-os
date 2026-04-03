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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_bad_magic() {
        let e = ParseError::BadMagic { magic: 0xDEADBEEF };
        assert_eq!(e.to_string(), "not a Mach-O file (bad magic: 0xdeadbeef)");
    }

    #[test]
    fn display_unexpected_eof() {
        let e = ParseError::UnexpectedEof {
            offset: 100,
            needed: 64,
            available: 32,
        };
        let s = e.to_string();
        assert!(s.contains("100"), "should contain offset");
        assert!(s.contains("64"), "should contain needed");
        assert!(s.contains("32"), "should contain available");
    }

    #[test]
    fn display_load_command_too_small() {
        let e = ParseError::LoadCommandTooSmall { size: 4, min: 8 };
        let s = e.to_string();
        assert!(s.contains("4") && s.contains("8"));
    }

    #[test]
    fn display_too_many_sections() {
        let e = ParseError::TooManySections { count: 999 };
        assert!(e.to_string().contains("999"));
    }

    #[test]
    fn display_other() {
        let e = ParseError::Other("custom message".into());
        assert_eq!(e.to_string(), "custom message");
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: ParseError = io_err.into();
        assert!(e.to_string().contains("file missing"));
    }
}
