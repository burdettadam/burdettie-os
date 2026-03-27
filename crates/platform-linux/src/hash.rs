//! SHA-256 via pure-Rust `sha2` crate.

use platform_core::hash::Hasher;
use sha2::Digest;

pub struct LinuxHasher;

impl Hasher for LinuxHasher {
    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        sha2::Sha256::digest(data).into()
    }
}
