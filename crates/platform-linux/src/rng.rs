//! Linux CSPRNG via getrandom(2) or /dev/urandom fallback.

use platform_core::rng::Rng;

pub struct LinuxRng;

impl Rng for LinuxRng {
    fn fill_bytes(&self, buf: &mut [u8]) {
        fill_random(buf);
    }
}

#[cfg(target_os = "linux")]
fn fill_random(buf: &mut [u8]) {
    // getrandom(2) — available since Linux 3.17
    let ret = unsafe { libc::getrandom(buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0) };
    if ret < 0 || (ret as usize) < buf.len() {
        // Fallback: read from /dev/urandom
        fallback_urandom(buf);
    }
}

#[cfg(not(target_os = "linux"))]
fn fill_random(buf: &mut [u8]) {
    // Fallback for compilation on macOS: use arc4random_buf.
    unsafe {
        libc::arc4random_buf(buf.as_mut_ptr() as *mut libc::c_void, buf.len());
    }
}

#[cfg(target_os = "linux")]
fn fallback_urandom(buf: &mut [u8]) {
    use std::io::Read;
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(buf);
    }
}
