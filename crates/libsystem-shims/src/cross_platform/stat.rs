//! Cross-platform `stat`/`lstat`/`fstat` shims.
//!
//! macOS and Linux have different `struct stat` layouts:
//!
//! | Field         | macOS offset | Linux aarch64 offset |
//! |---------------|-------------|---------------------|
//! | st_dev        | 0  (i32)    | 0   (u64)           |
//! | st_mode       | 4  (u16)    | 16  (u32)           |
//! | st_nlink      | 6  (u16)    | 20  (u32)           |
//! | st_ino        | 8  (u64)    | 8   (u64)           |
//! | st_uid        | 16 (u32)    | 24  (u32)           |
//! | st_gid        | 20 (u32)    | 28  (u32)           |
//! | st_rdev       | 24 (i32)    | 32  (u64)           |
//! | st_atimespec  | 32 (16B)    | 64  (16B)           |
//! | st_mtimespec  | 48 (16B)    | 80  (16B)           |
//! | st_ctimespec  | 64 (16B)    | 96  (16B)           |
//! | st_birthtimespec | 80 (16B) | (no equivalent)     |
//! | st_size       | 96 (i64)    | 48  (i64)           |
//! | st_blocks     | 104 (i64)   | 56  (i64)           |
//! | st_blksize    | 112 (i32)   | 112 (u32)           |
//! | st_flags      | 116 (u32)   | (no equivalent)     |
//! | st_gen        | 120 (u32)   | (no equivalent)     |
//!
//! macOS struct stat (INODE64) is 144 bytes.
//! Linux struct stat (aarch64) is 128 bytes.
//!
//! These shims call Linux's `stat`/`lstat`/`fstat`, then write the
//! results into the caller's buffer in macOS layout.

use crate::table::ShimTable;

pub struct StatShims;

impl StatShims {
    pub fn install(table: &mut ShimTable) {
        #[cfg(not(target_os = "macos"))]
        {
            use crate::register_shims;
            register_shims!(
                table,
                (
                    "_stat",
                    shim_stat as *const () as u64,
                    "stat macOS→Linux struct"
                ),
                (
                    "_stat$INODE64",
                    shim_stat as *const () as u64,
                    "stat$INODE64 macOS→Linux struct"
                ),
                (
                    "_lstat",
                    shim_lstat as *const () as u64,
                    "lstat macOS→Linux struct"
                ),
                (
                    "_lstat$INODE64",
                    shim_lstat as *const () as u64,
                    "lstat$INODE64 macOS→Linux struct"
                ),
                (
                    "_fstat",
                    shim_fstat as *const () as u64,
                    "fstat macOS→Linux struct"
                ),
                (
                    "_fstat$INODE64",
                    shim_fstat as *const () as u64,
                    "fstat$INODE64 macOS→Linux struct"
                ),
                (
                    "_fstatat",
                    shim_fstatat as *const () as u64,
                    "fstatat macOS→Linux struct"
                ),
                (
                    "_fstatat$INODE64",
                    shim_fstatat as *const () as u64,
                    "fstatat$INODE64 macOS→Linux struct"
                ),
                (
                    "_statfs",
                    shim_statfs as *const () as u64,
                    "statfs macOS→Linux struct"
                ),
                (
                    "_statfs$INODE64",
                    shim_statfs as *const () as u64,
                    "statfs$INODE64 macOS→Linux struct"
                ),
                (
                    "_fstatfs",
                    shim_fstatfs as *const () as u64,
                    "fstatfs macOS→Linux struct"
                ),
                (
                    "_fstatfs$INODE64",
                    shim_fstatfs as *const () as u64,
                    "fstatfs$INODE64 macOS→Linux struct"
                ),
            );
        }
        #[cfg(target_os = "macos")]
        let _ = table;
    }
}

/// macOS `struct stat` (INODE64 variant, 144 bytes).
#[cfg(not(target_os = "macos"))]
#[repr(C)]
struct DarwinStat {
    st_dev: i32,                // 0
    st_mode: u16,               // 4
    st_nlink: u16,              // 6
    st_ino: u64,                // 8
    st_uid: u32,                // 16
    st_gid: u32,                // 20
    st_rdev: i32,               // 24
    _pad0: i32,                 // 28 (padding for alignment)
    st_atimespec: [i64; 2],     // 32 (tv_sec, tv_nsec)
    st_mtimespec: [i64; 2],     // 48
    st_ctimespec: [i64; 2],     // 64
    st_birthtimespec: [i64; 2], // 80
    st_size: i64,               // 96
    st_blocks: i64,             // 104
    st_blksize: i32,            // 112
    st_flags: u32,              // 116
    st_gen: u32,                // 120
    _pad1: i32,                 // 124
    _reserved: [i64; 2],        // 128
}

/// Convert a Linux `libc::stat` to a macOS `DarwinStat` layout.
#[cfg(not(target_os = "macos"))]
unsafe fn linux_stat_to_darwin(linux: &libc::stat, darwin: *mut DarwinStat) {
    unsafe {
        let d = &mut *darwin;
        d.st_dev = linux.st_dev as i32;
        d.st_mode = linux.st_mode as u16;
        d.st_nlink = linux.st_nlink as u16;
        d.st_ino = linux.st_ino;
        d.st_uid = linux.st_uid;
        d.st_gid = linux.st_gid;
        d.st_rdev = linux.st_rdev as i32;
        d._pad0 = 0;
        d.st_atimespec = [linux.st_atime, linux.st_atime_nsec];
        d.st_mtimespec = [linux.st_mtime, linux.st_mtime_nsec];
        d.st_ctimespec = [linux.st_ctime, linux.st_ctime_nsec];
        // Linux doesn't have birth time — use ctime as fallback
        d.st_birthtimespec = [linux.st_ctime, linux.st_ctime_nsec];
        d.st_size = linux.st_size;
        d.st_blocks = linux.st_blocks;
        d.st_blksize = linux.st_blksize as i32;
        d.st_flags = 0; // no BSD flags on Linux
        d.st_gen = 0;
        d._pad1 = 0;
        d._reserved = [0; 2];
    }
}

/// `int stat(const char *path, struct stat *buf)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_stat(path: *const libc::c_char, darwin_buf: *mut DarwinStat) -> libc::c_int {
    if darwin_buf.is_null() {
        return -1;
    }
    let mut linux_buf: libc::stat = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::stat(path, &mut linux_buf) };
    if ret == 0 {
        unsafe {
            linux_stat_to_darwin(&linux_buf, darwin_buf);
        }
    }
    ret
}

/// `int lstat(const char *path, struct stat *buf)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_lstat(path: *const libc::c_char, darwin_buf: *mut DarwinStat) -> libc::c_int {
    if darwin_buf.is_null() {
        return -1;
    }
    let mut linux_buf: libc::stat = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::lstat(path, &mut linux_buf) };
    if ret == 0 {
        unsafe {
            linux_stat_to_darwin(&linux_buf, darwin_buf);
        }
    }
    ret
}

/// `int fstat(int fd, struct stat *buf)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fstat(fd: libc::c_int, darwin_buf: *mut DarwinStat) -> libc::c_int {
    if darwin_buf.is_null() {
        return -1;
    }
    let mut linux_buf: libc::stat = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::fstat(fd, &mut linux_buf) };
    if ret == 0 {
        unsafe {
            linux_stat_to_darwin(&linux_buf, darwin_buf);
        }
    }
    ret
}

/// `int fstatat(int fd, const char *path, struct stat *buf, int flag)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fstatat(
    fd: libc::c_int,
    path: *const libc::c_char,
    darwin_buf: *mut DarwinStat,
    flag: libc::c_int,
) -> libc::c_int {
    if darwin_buf.is_null() {
        return -1;
    }
    let mut linux_buf: libc::stat = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::fstatat(fd, path, &mut linux_buf, flag) };
    if ret == 0 {
        unsafe {
            linux_stat_to_darwin(&linux_buf, darwin_buf);
        }
    }
    ret
}

// ── statfs / fstatfs struct translation ──────────────────────────────
//
// macOS `struct statfs` is 2168 bytes with 1024-char path buffers.
// Linux `struct statfs` is 120 bytes with no path fields.
//
// | Field          | macOS offset (size) | Linux aarch64 offset (size) |
// |----------------|--------------------|-----------------------------|
// | f_bsize        | 0  (u32)           | 8  (i64)                    |
// | f_iosize       | 4  (i32)           | (none)                      |
// | f_blocks       | 8  (u64)           | 16 (u64)                    |
// | f_bfree        | 16 (u64)           | 24 (u64)                    |
// | f_bavail       | 24 (u64)           | 32 (u64)                    |
// | f_files        | 32 (u64)           | 40 (u64)                    |
// | f_ffree        | 40 (u64)           | 48 (u64)                    |
// | f_fsid         | 48 (8B)            | 56 (8B)                     |
// | f_owner        | 56 (u32)           | (none)                      |
// | f_type         | 60 (u32)           | 0  (i64)                    |
// | f_flags        | 64 (u32)           | 80 (i64)                    |
// | f_fssubtype    | 68 (u32)           | (none)                      |
// | f_fstypename   | 72 (char[16])      | (none)                      |
// | f_mntonname    | 88 (char[1024])    | (none)                      |
// | f_mntfromname  | 1112 (char[1024])  | (none)                      |
// | sizeof         | 2168               | 120                         |

/// macOS `struct statfs` (2168 bytes).
#[cfg(not(target_os = "macos"))]
#[repr(C)]
struct DarwinStatfs {
    f_bsize: u32,              // 0
    f_iosize: i32,             // 4
    f_blocks: u64,             // 8
    f_bfree: u64,              // 16
    f_bavail: u64,             // 24
    f_files: u64,              // 32
    f_ffree: u64,              // 40
    f_fsid: [i32; 2],          // 48
    f_owner: u32,              // 56
    f_type: u32,               // 60
    f_flags: u32,              // 64
    f_fssubtype: u32,          // 68
    f_fstypename: [u8; 16],    // 72
    f_mntonname: [u8; 1024],   // 88
    f_mntfromname: [u8; 1024], // 1112
    f_flags_ext: u32,          // 2136
    f_reserved: [u32; 7],      // 2140
}

/// Convert a Linux `libc::statfs` to a macOS `DarwinStatfs` layout.
#[cfg(not(target_os = "macos"))]
unsafe fn linux_statfs_to_darwin(linux: &libc::statfs, darwin: *mut DarwinStatfs) {
    unsafe {
        let d = &mut *darwin;
        // Zero everything first (especially the 1024-char path buffers)
        std::ptr::write_bytes(darwin as *mut u8, 0, std::mem::size_of::<DarwinStatfs>());
        d.f_bsize = linux.f_bsize as u32;
        d.f_iosize = linux.f_bsize as i32; // Linux has no iosize, use bsize
        d.f_blocks = linux.f_blocks;
        d.f_bfree = linux.f_bfree;
        d.f_bavail = linux.f_bavail;
        d.f_files = linux.f_files;
        d.f_ffree = linux.f_ffree;
        // On Linux, fsid_t is { __val: [i32; 2] } — copy the raw bytes.
        d.f_fsid = std::mem::transmute::<libc::fsid_t, [i32; 2]>(linux.f_fsid);
        d.f_owner = 0; // no Linux equivalent
        d.f_type = linux.f_type as u32;
        // Linux glibc statfs doesn't expose f_flags directly;
        // leave it as 0 (already zeroed above).
        d.f_fssubtype = 0;
        // f_fstypename: leave zeroed (could be populated from /proc/mounts)
        // f_mntonname: leave zeroed
        // f_mntfromname: leave zeroed
    }
}

/// `int statfs(const char *path, struct statfs *buf)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_statfs(path: *const libc::c_char, darwin_buf: *mut DarwinStatfs) -> libc::c_int {
    if darwin_buf.is_null() {
        return -1;
    }
    let mut linux_buf: libc::statfs = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::statfs(path, &mut linux_buf) };
    if ret == 0 {
        unsafe {
            linux_statfs_to_darwin(&linux_buf, darwin_buf);
        }
    }
    ret
}

/// `int fstatfs(int fd, struct statfs *buf)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fstatfs(fd: libc::c_int, darwin_buf: *mut DarwinStatfs) -> libc::c_int {
    if darwin_buf.is_null() {
        return -1;
    }
    let mut linux_buf: libc::statfs = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::fstatfs(fd, &mut linux_buf) };
    if ret == 0 {
        unsafe {
            linux_statfs_to_darwin(&linux_buf, darwin_buf);
        }
    }
    ret
}
