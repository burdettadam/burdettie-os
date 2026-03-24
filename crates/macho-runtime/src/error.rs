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
