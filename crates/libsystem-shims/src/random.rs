//! Random number generation shims.
//!
//! Provides `arc4random`, `arc4random_buf`, and `arc4random_uniform` using
//! the platform's CSPRNG. On macOS these delegate to libc. On Linux they
//! use `getrandom(2)`.

use crate::table::ShimTable;

/// Installs randomness-related shims.
pub struct RandomShims;

impl RandomShims {
    /// Register random shims in the table.
    pub fn install(table: &mut ShimTable) {
        table.register(
            "_arc4random",
            shim_arc4random as u64,
            "arc4random (portable)",
        );
        table.register(
            "_arc4random_buf",
            shim_arc4random_buf as u64,
            "arc4random_buf (portable)",
        );
        table.register(
            "_arc4random_uniform",
            shim_arc4random_uniform as u64,
            "arc4random_uniform (portable)",
        );

        log::debug!("RandomShims: registered 3 random shims");
    }
}

/// `uint32_t arc4random(void)` — returns a random 32-bit value.
extern "C" fn shim_arc4random() -> u32 {
    let mut buf = [0u8; 4];
    fill_random(&mut buf);
    u32::from_ne_bytes(buf)
}

/// `void arc4random_buf(void *buf, size_t nbytes)` — fills buffer with random bytes.
extern "C" fn shim_arc4random_buf(buf: *mut u8, nbytes: usize) {
    if buf.is_null() || nbytes == 0 {
        return;
    }
    // SAFETY: caller guarantees buf points to at least nbytes of writable memory.
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, nbytes) };
    fill_random(slice);
}

/// `uint32_t arc4random_uniform(uint32_t upper_bound)` — returns a random
/// value uniformly distributed in [0, upper_bound).
extern "C" fn shim_arc4random_uniform(upper_bound: u32) -> u32 {
    if upper_bound < 2 {
        return 0;
    }
    // Rejection sampling to avoid modulo bias.
    let min = (u32::MAX - upper_bound + 1) % upper_bound;
    loop {
        let val = shim_arc4random();
        if val >= min {
            return val % upper_bound;
        }
    }
}

/// Fill a buffer with cryptographically-secure random bytes.
fn fill_random(buf: &mut [u8]) {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: arc4random_buf is always safe and never fails.
        unsafe {
            libc::arc4random_buf(buf.as_mut_ptr().cast(), buf.len());
        }
    }
    #[cfg(target_os = "linux")]
    {
        // SAFETY: getrandom with flags=0 blocks until entropy is available.
        unsafe {
            libc::getrandom(buf.as_mut_ptr().cast(), buf.len(), 0);
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // Fallback: use std's thread-local RNG via Instant entropy.
        // This is not ideal for production but allows compilation.
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let mut offset = 0;
        while offset < buf.len() {
            let s = RandomState::new();
            let val = s.build_hasher().finish().to_ne_bytes();
            let chunk = (buf.len() - offset).min(8);
            buf[offset..offset + chunk].copy_from_slice(&val[..chunk]);
            offset += chunk;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_shims_registered() {
        let mut table = ShimTable::new();
        RandomShims::install(&mut table);
        assert!(table.resolve("_arc4random").is_some());
        assert!(table.resolve("_arc4random_buf").is_some());
        assert!(table.resolve("_arc4random_uniform").is_some());
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn arc4random_returns_values() {
        // Very unlikely (but not impossible) to get exactly 0 twice.
        let v1 = shim_arc4random();
        let v2 = shim_arc4random();
        assert!(v1 != 0 || v2 != 0, "arc4random should return non-zero values");
    }

    #[test]
    fn arc4random_buf_fills_buffer() {
        let mut buf = [0u8; 32];
        shim_arc4random_buf(buf.as_mut_ptr(), buf.len());
        // At least some bytes should be non-zero.
        assert!(buf.iter().any(|&b| b != 0), "buffer should have non-zero bytes");
    }

    #[test]
    fn arc4random_uniform_in_range() {
        for _ in 0..100 {
            let val = shim_arc4random_uniform(10);
            assert!(val < 10, "arc4random_uniform(10) returned {val}");
        }
    }

    #[test]
    fn arc4random_uniform_zero_and_one() {
        assert_eq!(shim_arc4random_uniform(0), 0);
        assert_eq!(shim_arc4random_uniform(1), 0);
    }

    #[test]
    fn arc4random_buf_null_safe() {
        // Should not crash.
        shim_arc4random_buf(std::ptr::null_mut(), 10);
        shim_arc4random_buf(std::ptr::null_mut(), 0);
    }
}
