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
