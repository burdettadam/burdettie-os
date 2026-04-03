//! Runtime error types.

use std::path::PathBuf;

/// Errors that can occur during runtime execution.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("parse error: {0}")]
    Parse(#[from] macho_parse::ParseError),

    #[error("execution error: {0}")]
    Exec(#[from] macho_exec::ExecError),

    #[error("security denied: {}", reasons.join("; "))]
    SecurityDenied { reasons: Vec<String> },

    #[error("encrypted binary (crypt_id={crypt_id})")]
    EncryptedBinary { crypt_id: u32 },

    #[error("no arm64 slice in fat binary")]
    NoArm64Slice,

    #[error("binary has no entry point")]
    NoEntryPoint,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let err = RuntimeError::Io {
            path: PathBuf::from("/tmp/test.bin"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/test.bin"), "should contain path: {msg}");
        assert!(msg.contains("file not found"), "should contain cause: {msg}");
    }

    #[test]
    fn security_denied_display() {
        let err = RuntimeError::SecurityDenied {
            reasons: vec!["unsigned".to_string(), "no entitlements".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("unsigned"), "{msg}");
        assert!(msg.contains("no entitlements"), "{msg}");
        assert!(msg.contains("; "), "reasons should be joined with '; ': {msg}");
    }

    #[test]
    fn encrypted_binary_display() {
        let err = RuntimeError::EncryptedBinary { crypt_id: 42 };
        let msg = err.to_string();
        assert!(msg.contains("crypt_id=42"), "{msg}");
    }

    #[test]
    fn no_arm64_slice_display() {
        let err = RuntimeError::NoArm64Slice;
        assert_eq!(err.to_string(), "no arm64 slice in fat binary");
    }

    #[test]
    fn no_entry_point_display() {
        let err = RuntimeError::NoEntryPoint;
        assert_eq!(err.to_string(), "binary has no entry point");
    }

    #[test]
    fn runtime_error_is_debug() {
        let variants: Vec<Box<dyn std::fmt::Debug>> = vec![
            Box::new(RuntimeError::Io {
                path: PathBuf::from("/x"),
                source: std::io::Error::new(std::io::ErrorKind::Other, "oops"),
            }),
            Box::new(RuntimeError::SecurityDenied {
                reasons: vec!["r".into()],
            }),
            Box::new(RuntimeError::EncryptedBinary { crypt_id: 1 }),
            Box::new(RuntimeError::NoArm64Slice),
            Box::new(RuntimeError::NoEntryPoint),
        ];
        for v in &variants {
            let dbg = format!("{:?}", v);
            assert!(!dbg.is_empty());
        }
    }
}
