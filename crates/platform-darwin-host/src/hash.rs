//! SHA-256 via pure-Rust `sha2` crate (no CommonCrypto dependency).

use platform_core::hash::Hasher;
use sha2::Digest;

pub struct DarwinHasher;

impl Hasher for DarwinHasher {
    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        sha2::Sha256::digest(data).into()
    }
}
