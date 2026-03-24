//! macOS SHA-256 via CommonCrypto CC_SHA256.

use platform_core::hash::Hasher;

pub struct DarwinHasher;

impl Hasher for DarwinHasher {
    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        extern "C" {
            fn CC_SHA256(data: *const u8, len: u32, md: *mut u8) -> *mut u8;
        }
        let mut hash = [0u8; 32];
        unsafe {
            CC_SHA256(data.as_ptr(), data.len() as u32, hash.as_mut_ptr());
        }
        hash
    }
}
