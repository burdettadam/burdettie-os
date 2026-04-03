//! SHA-256 via pure-Rust `sha2` crate (no CommonCrypto dependency).

use platform_core::hash::Hasher;
use sha2::Digest;

pub struct DarwinHasher;

impl Hasher for DarwinHasher {
    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        sha2::Sha256::digest(data).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty() {
        let h = DarwinHasher;
        let digest = h.sha256(b"");
        // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let expected: [u8; 32] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(digest, expected);
    }

    #[test]
    fn sha256_hello() {
        let h = DarwinHasher;
        let digest = h.sha256(b"hello");
        // 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let expected: [u8; 32] = [
            0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9,
            0xe2, 0x9e, 0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62,
            0x93, 0x8b, 0x98, 0x24,
        ];
        assert_eq!(digest, expected);
    }

    #[test]
    fn sha256_deterministic() {
        let h = DarwinHasher;
        let a = h.sha256(b"deterministic test");
        let b = h.sha256(b"deterministic test");
        assert_eq!(a, b);
    }

    #[test]
    fn sha256_different_inputs() {
        let h = DarwinHasher;
        let a = h.sha256(b"input A");
        let b = h.sha256(b"input B");
        assert_ne!(a, b);
    }
}
