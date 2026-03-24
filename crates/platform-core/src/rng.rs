//! Cryptographically secure random number generation.

/// Fill a buffer with random bytes.
pub trait Rng: Send + Sync {
    fn fill_bytes(&self, buf: &mut [u8]);
}
