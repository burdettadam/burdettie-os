//! Cross-platform `readdir` / `readdir_r` shims.
//!
//! macOS and Linux have different `struct dirent` layouts:
//!
//! | Field     | macOS offset (size) | Linux aarch64 offset (size) |
//! |-----------|--------------------|-----------------------------|
//! | d_ino     | 0  (u64)           | 0  (u64)                    |
//! | d_seekoff | 8  (u64)           | 8  (i64 as d_off)           |
//! | d_reclen  | 16 (u16)           | 16 (u16)                    |
//! | d_namlen  | 18 (u16)           | (none)                      |
//! | d_type    | 20 (u8)            | 18 (u8)                     |
//! | d_name    | 21 (char[1024])    | 19 (char[256])              |
//! | sizeof    | 1048               | 280                         |
//!
//! The 2-byte shift caused by macOS's `d_namlen` field means a macOS
//! binary reading `d_name` at offset 21 from a Linux dirent pointer
//! gets garbage (the name shifted by 2).

use crate::table::ShimTable;

pub struct DirentShims;

impl DirentShims {
    pub fn install(table: &mut ShimTable) {
        #[cfg(not(target_os = "macos"))]
        {
            use crate::register_shims;
            register_shims!(table,
                ("_readdir", shim_readdir as *const () as u64, "readdir macOS→Linux dirent"),
                ("_readdir$INODE64", shim_readdir as *const () as u64, "readdir$INODE64 macOS→Linux dirent"),
            );
        }
        #[cfg(target_os = "macos")]
        let _ = table;
    }
}

/// macOS `struct dirent` (1048 bytes, 8-byte aligned).
#[cfg(not(target_os = "macos"))]
const DARWIN_DIRENT_SIZE: usize = 1048;

#[cfg(not(target_os = "macos"))]
#[repr(C)]
struct DarwinDirent {
    d_ino: u64,            // 0
    d_seekoff: u64,        // 8
    d_reclen: u16,         // 16
    d_namlen: u16,         // 18
    d_type: u8,            // 20
    d_name: [u8; 1024],    // 21
    _pad: [u8; 3],         // 1045 (pad to 1048)
}

/// Thread-local buffer for translated dirent results.
/// `readdir()` returns a pointer valid until the next `readdir()` call
/// on the same DIR stream — a thread-local buffer satisfies this contract.
#[cfg(not(target_os = "macos"))]
std::thread_local! {
    static DIRENT_BUF: std::cell::UnsafeCell<[u8; DARWIN_DIRENT_SIZE]> =
        std::cell::UnsafeCell::new([0u8; DARWIN_DIRENT_SIZE]);
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_readdir(dirp: *mut libc::DIR) -> *mut DarwinDirent {
    if dirp.is_null() {
        return std::ptr::null_mut();
    }

    let linux_ent = unsafe { libc::readdir(dirp) };
    if linux_ent.is_null() {
        return std::ptr::null_mut();
    }

    DIRENT_BUF.with(|buf| {
        let darwin_ptr = buf.get() as *mut DarwinDirent;
        unsafe {
            let linux = &*linux_ent;

            // Zero the buffer first
            std::ptr::write_bytes(darwin_ptr as *mut u8, 0, DARWIN_DIRENT_SIZE);

            let d = &mut *darwin_ptr;
            d.d_ino = linux.d_ino;
            d.d_seekoff = linux.d_off as u64;
            d.d_reclen = DARWIN_DIRENT_SIZE as u16;

            // Compute name length from the C string
            let name_ptr = linux.d_name.as_ptr();
            let name_len = libc::strlen(name_ptr);
            d.d_namlen = name_len as u16;
            d.d_type = linux.d_type;

            // Copy the name (including null terminator)
            let copy_len = std::cmp::min(name_len + 1, 1024);
            std::ptr::copy_nonoverlapping(
                name_ptr as *const u8,
                d.d_name.as_mut_ptr(),
                copy_len,
            );
        }
        darwin_ptr
    })
}
