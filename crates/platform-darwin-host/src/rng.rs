//! macOS CSPRNG via arc4random_buf.

use platform_core::rng::Rng;

pub struct DarwinRng;

impl Rng for DarwinRng {
    fn fill_bytes(&self, buf: &mut [u8]) {
        unsafe {
            libc::arc4random_buf(buf.as_mut_ptr() as *mut libc::c_void, buf.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_bytes_not_all_zero() {
        let rng = DarwinRng;
        let mut buf = [0u8; 64];
        rng.fill_bytes(&mut buf);
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn fill_bytes_zero_length() {
        let rng = DarwinRng;
        let mut buf = [];
        rng.fill_bytes(&mut buf); // should not crash
    }

    #[test]
    fn fill_bytes_different_each_call() {
        let rng = DarwinRng;
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut a);
        rng.fill_bytes(&mut b);
        assert_ne!(a, b);
    }
}
