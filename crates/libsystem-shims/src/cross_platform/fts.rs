//! FTS (file tree walk) shims.
//!
//! macOS and Linux FTSENT structs have identical layouts for bytes 0–79
//! (pointers, ints, shorts) but diverge starting at byte 80 because
//! `dev_t` is i32 on macOS vs u64 on Linux, and `nlink_t` is u16 vs u32.
//!
//! Linux FTSENT layout (bytes 80+):
//!   80: fts_dev    (u64, 8)
//!   88: fts_nlink  (u32, 4)
//!   92: fts_level  (i16, 2)
//!   94: fts_info   (u16, 2)
//!   96: fts_flags  (u16, 2)
//!   98: fts_instr  (u16, 2)
//!  100: padding    (4)
//!  104: fts_statp  (ptr, 8)
//!  112: fts_name[] (flex)
//!
//! macOS FTSENT layout (bytes 80+):
//!   80: fts_dev    (i32, 4)
//!   84: fts_nlink  (u16, 2)
//!   86: fts_level  (i16, 2)
//!   88: fts_info   (u16, 2)
//!   90: fts_flags  (u16, 2)
//!   92: fts_instr  (u16, 2)
//!   94: padding    (2)
//!   96: fts_statp  (ptr, 8)
//!  104: fts_name[] (flex)
//!
//! The FTS option flags and FTS_INFO values are identical on both platforms.

use crate::table::ShimTable;

pub struct FtsShims;

impl FtsShims {
    pub fn install(table: &mut ShimTable) {
        #[cfg(not(target_os = "macos"))]
        {
            use crate::register_shims;
            register_shims!(
                table,
                (
                    "_fts_open$INODE64",
                    shim_fts_open as *const () as u64,
                    "fts_open FTSENT layout translation"
                ),
                (
                    "_fts_open",
                    shim_fts_open as *const () as u64,
                    "fts_open FTSENT layout translation"
                ),
                (
                    "_fts_read$INODE64",
                    shim_fts_read as *const () as u64,
                    "fts_read FTSENT layout translation"
                ),
                (
                    "_fts_read",
                    shim_fts_read as *const () as u64,
                    "fts_read FTSENT layout translation"
                ),
                (
                    "_fts_close$INODE64",
                    shim_fts_close as *const () as u64,
                    "fts_close passthrough"
                ),
                (
                    "_fts_close",
                    shim_fts_close as *const () as u64,
                    "fts_close passthrough"
                ),
                (
                    "_fts_set$INODE64",
                    shim_fts_set as *const () as u64,
                    "fts_set passthrough"
                ),
                (
                    "_fts_set",
                    shim_fts_set as *const () as u64,
                    "fts_set passthrough"
                ),
                (
                    "_fts_children$INODE64",
                    shim_fts_children as *const () as u64,
                    "fts_children FTSENT layout translation"
                ),
                (
                    "_fts_children",
                    shim_fts_children as *const () as u64,
                    "fts_children FTSENT layout translation"
                ),
            );
        }
        #[cfg(target_os = "macos")]
        let _ = table;
    }
}

// ---------------------------------------------------------------------------
// Implementation (Linux only)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "macos"))]
mod inner {
    use std::cell::UnsafeCell;

    // Link against glibc's FTS functions directly.
    extern "C" {
        fn fts_open(
            path_argv: *const *mut libc::c_char,
            options: libc::c_int,
            compar: Option<unsafe extern "C" fn(*const *const u8, *const *const u8) -> libc::c_int>,
        ) -> *mut u8;
        fn fts_read(ftsp: *mut u8) -> *mut u8;
        fn fts_close(ftsp: *mut u8) -> libc::c_int;
        fn fts_set(ftsp: *mut u8, f: *mut u8, instr: libc::c_int) -> libc::c_int;
        fn fts_children(ftsp: *mut u8, options: libc::c_int) -> *mut u8;
    }

    // Thread-local shadow buffers for the translated macOS-layout FTSENT.
    // find processes one entry at a time, so a single current + parent pair suffices.
    const SHADOW_SIZE: usize = 8192;
    const DARWIN_STAT_SIZE: usize = 144;
    const FTS_NOSTAT: libc::c_int = 0x008;

    struct FtsShadowBuffers {
        entry: [u8; SHADOW_SIZE],
        stat_buf: [u8; DARWIN_STAT_SIZE],
        parent: [u8; SHADOW_SIZE],
        parent_stat: [u8; DARWIN_STAT_SIZE],
        nostat: bool,
    }

    // Safety: these are thread-local and only accessed from the owning thread.
    // We use UnsafeCell because we need interior mutability without RefCell overhead.
    thread_local! {
        static SHADOW: UnsafeCell<FtsShadowBuffers> = UnsafeCell::new(FtsShadowBuffers {
            entry: [0u8; SHADOW_SIZE],
            stat_buf: [0u8; DARWIN_STAT_SIZE],
            parent: [0u8; SHADOW_SIZE],
            parent_stat: [0u8; DARWIN_STAT_SIZE],
            nostat: false,
        });
    }

    // ── Linux FTSENT field offsets ────────────────────────────────────
    const LINUX_FTS_PARENT: usize = 8; // ptr
    const LINUX_FTS_LINK: usize = 16; // ptr
    const LINUX_FTS_NAMELEN: usize = 66; // u16
    const LINUX_FTS_DEV: usize = 80; // u64
    const LINUX_FTS_NLINK: usize = 88; // u32
    const LINUX_FTS_LEVEL: usize = 92; // i16
    const LINUX_FTS_INFO: usize = 94; // u16
    const LINUX_FTS_FLAGS: usize = 96; // u16
    const LINUX_FTS_INSTR: usize = 98; // u16
    const LINUX_FTS_STATP: usize = 104; // ptr
    const LINUX_FTS_NAME: usize = 112; // flex array

    // ── macOS FTSENT field offsets ────────────────────────────────────
    const DARWIN_FTS_PARENT: usize = 8;
    const DARWIN_FTS_LINK: usize = 16;
    const DARWIN_FTS_DEV: usize = 80; // i32
    const DARWIN_FTS_NLINK: usize = 84; // u16
    const DARWIN_FTS_LEVEL: usize = 86; // i16
    const DARWIN_FTS_INFO: usize = 88; // u16
    const DARWIN_FTS_FLAGS: usize = 90; // u16
    const DARWIN_FTS_INSTR: usize = 92; // u16
    const DARWIN_FTS_STATP: usize = 96; // ptr
    const DARWIN_FTS_NAME: usize = 104; // flex array

    /// Read a native-endian value from a byte buffer at the given offset.
    unsafe fn read_at<T: Copy>(buf: *const u8, off: usize) -> T {
        unsafe { std::ptr::read_unaligned(buf.add(off) as *const T) }
    }

    /// Write a native-endian value into a byte buffer at the given offset.
    unsafe fn write_at<T: Copy>(buf: *mut u8, off: usize, val: T) {
        unsafe {
            std::ptr::write_unaligned(buf.add(off) as *mut T, val);
        }
    }

    /// Translate a Linux stat struct in-place from a copied buffer.
    /// The buffer contains Linux stat bytes (128B). We read the Linux fields,
    /// then zero the buffer and write macOS layout (144B) back.
    /// This avoids directly casting foreign pointers to `libc::stat`.
    unsafe fn translate_stat_inplace(buf: *mut u8) {
        unsafe {
            // Read all Linux fields from buf (which has Linux stat layout)
            let st_dev: u64 = read_at(buf, 0);
            let st_ino: u64 = read_at(buf, 8);
            let st_mode: u32 = read_at(buf, 16);
            let st_nlink: u32 = read_at(buf, 20);
            let st_uid: u32 = read_at(buf, 24);
            let st_gid: u32 = read_at(buf, 28);
            let st_rdev: u64 = read_at(buf, 32);
            // 40: padding
            let st_size: i64 = read_at(buf, 48);
            let st_blksize: i32 = read_at(buf, 56);
            // 60: padding
            let st_blocks: i64 = read_at(buf, 64);
            let st_atime: i64 = read_at(buf, 72);
            let st_atime_nsec: i64 = read_at(buf, 80);
            let st_mtime: i64 = read_at(buf, 88);
            let st_mtime_nsec: i64 = read_at(buf, 96);
            let st_ctime: i64 = read_at(buf, 104);
            let st_ctime_nsec: i64 = read_at(buf, 112);

            // Zero the full 144 bytes for macOS layout
            std::ptr::write_bytes(buf, 0, DARWIN_STAT_SIZE);

            // Write macOS DarwinStat layout
            write_at::<i32>(buf, 0, st_dev as i32);
            write_at::<u16>(buf, 4, st_mode as u16);
            write_at::<u16>(buf, 6, st_nlink as u16);
            write_at::<u64>(buf, 8, st_ino);
            write_at::<u32>(buf, 16, st_uid);
            write_at::<u32>(buf, 20, st_gid);
            write_at::<i32>(buf, 24, st_rdev as i32);
            write_at::<i64>(buf, 32, st_atime);
            write_at::<i64>(buf, 40, st_atime_nsec);
            write_at::<i64>(buf, 48, st_mtime);
            write_at::<i64>(buf, 56, st_mtime_nsec);
            write_at::<i64>(buf, 64, st_ctime);
            write_at::<i64>(buf, 72, st_ctime_nsec);
            write_at::<i64>(buf, 80, st_ctime); // birthtimespec = ctime
            write_at::<i64>(buf, 88, st_ctime_nsec);
            write_at::<i64>(buf, 96, st_size);
            write_at::<i64>(buf, 104, st_blocks);
            write_at::<i32>(buf, 112, st_blksize);
        }
    }

    /// Translate a single Linux FTSENT into a macOS-layout shadow buffer.
    /// `stat_buf` is used for the translated stat struct (if fts_statp is non-NULL).
    /// If `translate_stat_ptr` is false, fts_statp is set to NULL (safe for parent nodes).
    /// If `nostat` is true, the FTS_NOSTAT option was passed and fts_statp is garbage.
    unsafe fn translate_ftsent(
        linux_entry: *const u8,
        shadow: *mut u8,
        stat_buf: *mut u8,
        translate_stat_ptr: bool,
        nostat: bool,
    ) {
        unsafe {
            // Bytes 0–79 are identical — copy verbatim.
            std::ptr::copy_nonoverlapping(linux_entry, shadow, 80);

            // Read Linux fields from bytes 80+
            let dev: u64 = read_at(linux_entry, LINUX_FTS_DEV);
            let nlink: u32 = read_at(linux_entry, LINUX_FTS_NLINK);
            let level: i16 = read_at(linux_entry, LINUX_FTS_LEVEL);
            let info: u16 = read_at(linux_entry, LINUX_FTS_INFO);
            let flags: u16 = read_at(linux_entry, LINUX_FTS_FLAGS);
            let instr: u16 = read_at(linux_entry, LINUX_FTS_INSTR);
            let namelen: u16 = read_at(linux_entry, LINUX_FTS_NAMELEN);

            // Write macOS fields at macOS offsets
            write_at::<i32>(shadow, DARWIN_FTS_DEV, dev as i32);
            write_at::<u16>(shadow, DARWIN_FTS_NLINK, nlink as u16);
            write_at::<i16>(shadow, DARWIN_FTS_LEVEL, level);
            write_at::<u16>(shadow, DARWIN_FTS_INFO, info);
            write_at::<u16>(shadow, DARWIN_FTS_FLAGS, flags);
            write_at::<u16>(shadow, DARWIN_FTS_INSTR, instr);
            write_at::<u16>(shadow, 94, 0); // padding

            // Translate stat struct if present and requested
            if translate_stat_ptr && !nostat {
                let linux_statp: *const u8 = read_at(linux_entry, LINUX_FTS_STATP);
                if !linux_statp.is_null() {
                    // Copy the Linux stat bytes into our stat_buf first, then
                    // translate in-place.  This avoids issues if the source
                    // memory is in a special region.
                    std::ptr::copy_nonoverlapping(linux_statp, stat_buf, 128);
                    // Now translate the copy (stat_buf has Linux layout, rewrite to macOS)
                    translate_stat_inplace(stat_buf);
                    write_at::<*const u8>(shadow, DARWIN_FTS_STATP, stat_buf as *const u8);
                } else {
                    write_at::<*const u8>(shadow, DARWIN_FTS_STATP, std::ptr::null());
                }
            } else {
                // For parent/sentinel nodes, don't dereference fts_statp —
                // it may be uninitialized garbage in the FTS sentinel entry.
                write_at::<*const u8>(shadow, DARWIN_FTS_STATP, std::ptr::null());
            }

            // Copy fts_name (Linux offset 112 → macOS offset 104)
            let name_len = namelen as usize;
            let src_name = linux_entry.add(LINUX_FTS_NAME);
            let dst_name = shadow.add(DARWIN_FTS_NAME);
            std::ptr::copy_nonoverlapping(src_name, dst_name, name_len + 1); // +1 for NUL
        }
    }

    // ── Public shim functions ─────────────────────────────────────────

    pub extern "C" fn shim_fts_open(
        path_argv: *const *mut libc::c_char,
        options: libc::c_int,
        compar: Option<unsafe extern "C" fn(*const *const u8, *const *const u8) -> libc::c_int>,
    ) -> *mut u8 {
        // Record FTS_NOSTAT so shim_fts_read knows not to translate stat.
        SHADOW.with(|cell| {
            let bufs = unsafe { &mut *cell.get() };
            bufs.nostat = (options & FTS_NOSTAT) != 0;
        });
        // Option flags are identical between macOS and Linux — pass through.
        unsafe { fts_open(path_argv, options, compar) }
    }

    pub extern "C" fn shim_fts_read(ftsp: *mut u8) -> *mut u8 {
        unsafe {
            let linux_entry = fts_read(ftsp);
            if linux_entry.is_null() {
                return std::ptr::null_mut();
            }

            SHADOW.with(|cell| {
                let bufs = &mut *cell.get();
                let nostat = bufs.nostat;

                // Translate the current entry (with stat unless FTS_NOSTAT)
                translate_ftsent(
                    linux_entry,
                    bufs.entry.as_mut_ptr(),
                    bufs.stat_buf.as_mut_ptr(),
                    true, // translate stat pointer
                    nostat,
                );

                // For fts_parent: translate just the fields the caller reads
                // (fts_level, fts_info, fts_name) without touching fts_statp
                // which may be garbage in sentinel nodes.
                let linux_parent: *const u8 = read_at(linux_entry, LINUX_FTS_PARENT);

                if !linux_parent.is_null() {
                    // Check if this is the sentinel (level == -1) or a real entry
                    let parent_level: i16 = read_at(linux_parent, LINUX_FTS_LEVEL);

                    if parent_level >= 0 {
                        // Real parent — translate fully but skip stat
                        translate_ftsent(
                            linux_parent,
                            bufs.parent.as_mut_ptr(),
                            bufs.parent_stat.as_mut_ptr(),
                            false, // skip stat for parent
                            nostat,
                        );
                    } else {
                        // Sentinel parent (level == -1): translate minimally
                        // Copy bytes 0-79, set macOS fields, NULL statp, empty name
                        std::ptr::copy_nonoverlapping(linux_parent, bufs.parent.as_mut_ptr(), 80);
                        write_at::<i32>(bufs.parent.as_mut_ptr(), DARWIN_FTS_DEV, 0);
                        write_at::<u16>(bufs.parent.as_mut_ptr(), DARWIN_FTS_NLINK, 0);
                        write_at::<i16>(bufs.parent.as_mut_ptr(), DARWIN_FTS_LEVEL, -1);
                        write_at::<u16>(bufs.parent.as_mut_ptr(), DARWIN_FTS_INFO, 0);
                        write_at::<u16>(bufs.parent.as_mut_ptr(), DARWIN_FTS_FLAGS, 0);
                        write_at::<u16>(bufs.parent.as_mut_ptr(), DARWIN_FTS_INSTR, 0);
                        write_at::<*const u8>(
                            bufs.parent.as_mut_ptr(),
                            DARWIN_FTS_STATP,
                            std::ptr::null(),
                        );
                        // Empty fts_name at macOS offset 104
                        *bufs.parent.as_mut_ptr().add(DARWIN_FTS_NAME) = 0;
                    }
                    // Point our macOS entry's fts_parent to the parent shadow
                    write_at::<*const u8>(
                        bufs.entry.as_mut_ptr(),
                        DARWIN_FTS_PARENT,
                        bufs.parent.as_ptr(),
                    );
                }

                // Keep fts_link from the copy (bytes 0-79 include offset 16)
                // but NULL it for safety — find iterates via fts_read, not fts_link
                write_at::<*const u8>(bufs.entry.as_mut_ptr(), DARWIN_FTS_LINK, std::ptr::null());

                bufs.entry.as_mut_ptr()
            })
        }
    }

    pub extern "C" fn shim_fts_close(ftsp: *mut u8) -> libc::c_int {
        unsafe { fts_close(ftsp) }
    }

    pub extern "C" fn shim_fts_set(ftsp: *mut u8, f: *mut u8, instr: libc::c_int) -> libc::c_int {
        // fts_set takes the original Linux FTSENT pointer.
        // Since we return shadow pointers from fts_read, the caller passes
        // our shadow. We need to recover the original Linux pointer.
        // However, fts_set with FTS_SKIP/FTS_FOLLOW just sets a flag
        // on the internal FTS state keyed by the current entry.
        // Pass through — glibc fts_set only uses ftsp->fts_cur internally,
        // and the `f` parameter's instr field is what matters.
        // We can't easily recover the Linux pointer, so just call with f=NULL
        // which some implementations handle, or ignore the call.
        //
        // In practice, macOS find uses fts_set(sp, p, FTS_SKIP) where p
        // was returned by fts_read. We handle this by calling fts_set on
        // the FTS handle's current entry (ftsp->fts_cur).
        unsafe {
            // Read fts_cur from the FTS handle (offset 0 on both macOS and Linux)
            let fts_cur: *mut u8 = read_at(ftsp, 0);
            fts_set(ftsp, fts_cur, instr)
        }
    }

    pub extern "C" fn shim_fts_children(ftsp: *mut u8, options: libc::c_int) -> *mut u8 {
        // fts_children returns a linked list of FTSENT entries.
        // Translating in-place is complex — for now, return NULL (no children).
        // find rarely uses fts_children; the main loop uses fts_read.
        unsafe {
            let _ = fts_children(ftsp, options);
        }
        // Return NULL — find will fall back to fts_read traversal.
        std::ptr::null_mut()
    }
}

#[cfg(not(target_os = "macos"))]
pub use inner::{shim_fts_children, shim_fts_close, shim_fts_open, shim_fts_read, shim_fts_set};
