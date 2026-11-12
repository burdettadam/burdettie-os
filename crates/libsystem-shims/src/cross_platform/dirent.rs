//! Cross-platform `readdir` / `scandir` / `alphasort` shims.
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
//!
//! `scandir` is particularly dangerous on the safe-list because it
//! returns heap-allocated arrays of `struct dirent*` — each only 280
//! bytes on Linux, but the guest reads up to 1048 bytes → SIGSEGV.

use crate::table::ShimTable;

pub struct DirentShims;

impl DirentShims {
    pub fn install(table: &mut ShimTable) {
        #[cfg(not(target_os = "macos"))]
        {
            use crate::register_shims;
            register_shims!(
                table,
                (
                    "_readdir",
                    shim_readdir as *const () as u64,
                    "readdir macOS→Linux dirent"
                ),
                (
                    "_readdir$INODE64",
                    shim_readdir as *const () as u64,
                    "readdir$INODE64 macOS→Linux dirent"
                ),
                (
                    "_scandir",
                    shim_scandir as *const () as u64,
                    "scandir macOS→Linux dirent"
                ),
                (
                    "_scandir$INODE64",
                    shim_scandir as *const () as u64,
                    "scandir$INODE64 macOS→Linux dirent"
                ),
                (
                    "_alphasort",
                    shim_alphasort as *const () as u64,
                    "alphasort macOS dirent"
                ),
                (
                    "_alphasort$INODE64",
                    shim_alphasort as *const () as u64,
                    "alphasort$INODE64 macOS dirent"
                ),
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
    d_ino: u64,         // 0
    d_seekoff: u64,     // 8
    d_reclen: u16,      // 16
    d_namlen: u16,      // 18
    d_type: u8,         // 20
    d_name: [u8; 1024], // 21
    _pad: [u8; 3],      // 1045 (pad to 1048)
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
            linux_to_darwin_dirent(&*linux_ent, darwin_ptr);
        }
        darwin_ptr
    })
}

/// Translate a Linux `struct dirent` into a macOS `DarwinDirent` at the given pointer.
#[cfg(not(target_os = "macos"))]
unsafe fn linux_to_darwin_dirent(linux: &libc::dirent, darwin_ptr: *mut DarwinDirent) {
    std::ptr::write_bytes(darwin_ptr as *mut u8, 0, DARWIN_DIRENT_SIZE);

    let d = &mut *darwin_ptr;
    d.d_ino = linux.d_ino;
    d.d_seekoff = linux.d_off as u64;
    d.d_reclen = DARWIN_DIRENT_SIZE as u16;

    let name_ptr = linux.d_name.as_ptr();
    let name_len = libc::strlen(name_ptr);
    d.d_namlen = name_len as u16;
    d.d_type = linux.d_type;

    let copy_len = std::cmp::min(name_len + 1, 1024);
    std::ptr::copy_nonoverlapping(name_ptr as *const u8, d.d_name.as_mut_ptr(), copy_len);
}

/// `scandir` shim: reads a directory using glibc opendir/readdir, translates
/// each entry to macOS layout, applies the guest's filter and comparator.
///
/// This cannot delegate to glibc's `scandir` because the filter/compar
/// callbacks are guest function pointers expecting macOS `struct dirent`.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_scandir(
    dirname: *const libc::c_char,
    namelist: *mut *mut *mut DarwinDirent,
    filter: Option<unsafe extern "C" fn(*const DarwinDirent) -> libc::c_int>,
    compar: Option<
        unsafe extern "C" fn(*const *const DarwinDirent, *const *const DarwinDirent) -> libc::c_int,
    >,
) -> libc::c_int {
    if dirname.is_null() || namelist.is_null() {
        unsafe {
            *libc::__errno_location() = libc::EFAULT;
        }
        return -1;
    }

    let dir = unsafe { libc::opendir(dirname) };
    if dir.is_null() {
        return -1; // errno already set by opendir
    }

    let mut entries: Vec<*mut DarwinDirent> = Vec::new();

    loop {
        // Reset errno to distinguish end-of-dir from error
        unsafe {
            *libc::__errno_location() = 0;
        }
        let linux_ent = unsafe { libc::readdir(dir) };
        if linux_ent.is_null() {
            if unsafe { *libc::__errno_location() } != 0 {
                // Error — free everything and return -1
                let saved_errno = unsafe { *libc::__errno_location() };
                for ptr in &entries {
                    unsafe {
                        libc::free(*ptr as *mut libc::c_void);
                    }
                }
                unsafe {
                    libc::closedir(dir);
                    *libc::__errno_location() = saved_errno;
                }
                return -1;
            }
            break; // End of directory
        }

        // Allocate a macOS-sized dirent on the heap
        let darwin_ptr = unsafe { libc::malloc(DARWIN_DIRENT_SIZE) as *mut DarwinDirent };
        if darwin_ptr.is_null() {
            for ptr in &entries {
                unsafe {
                    libc::free(*ptr as *mut libc::c_void);
                }
            }
            unsafe {
                libc::closedir(dir);
                *libc::__errno_location() = libc::ENOMEM;
            }
            return -1;
        }

        unsafe {
            linux_to_darwin_dirent(&*linux_ent, darwin_ptr);
        }

        // Apply the guest filter (if any)
        if let Some(f) = filter {
            if unsafe { f(darwin_ptr) } == 0 {
                unsafe {
                    libc::free(darwin_ptr as *mut libc::c_void);
                }
                continue;
            }
        }

        entries.push(darwin_ptr);
    }

    unsafe {
        libc::closedir(dir);
    }

    // Sort if comparator provided
    if let Some(cmp) = compar {
        entries.sort_by(|a, b| {
            let result = unsafe {
                cmp(
                    a as *const *mut DarwinDirent as *const *const DarwinDirent,
                    b as *const *mut DarwinDirent as *const *const DarwinDirent,
                )
            };
            result.cmp(&0)
        });
    }

    let count = entries.len();

    // Allocate the namelist array
    let list_ptr = unsafe {
        libc::malloc(count * std::mem::size_of::<*mut DarwinDirent>()) as *mut *mut DarwinDirent
    };
    if list_ptr.is_null() && count > 0 {
        for ptr in &entries {
            unsafe {
                libc::free(*ptr as *mut libc::c_void);
            }
        }
        unsafe {
            *libc::__errno_location() = libc::ENOMEM;
        }
        return -1;
    }

    // Copy pointers into the allocated array
    for (i, ptr) in entries.iter().enumerate() {
        unsafe {
            *list_ptr.add(i) = *ptr;
        }
    }

    unsafe {
        *namelist = list_ptr;
    }
    count as libc::c_int
}

/// `alphasort` shim for macOS-layout dirents.
/// Compares `d_name` fields which are at offset 21 in the macOS layout.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_alphasort(
    a: *const *const DarwinDirent,
    b: *const *const DarwinDirent,
) -> libc::c_int {
    unsafe {
        let da = &**a;
        let db = &**b;
        libc::strcoll(
            da.d_name.as_ptr() as *const libc::c_char,
            db.d_name.as_ptr() as *const libc::c_char,
        )
    }
}
