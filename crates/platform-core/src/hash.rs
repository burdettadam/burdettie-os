//! Cryptographic hashing abstraction.

/// Platform-independent cryptographic hasher.
pub trait Hasher: Send + Sync {
    /// Compute SHA-256 digest.
    fn sha256(&self, data: &[u8]) -> [u8; 32];
}
