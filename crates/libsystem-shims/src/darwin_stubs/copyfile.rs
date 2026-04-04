//! copyfile(3) and removefile(3) API shims.
//!
//! On macOS these live in libcopyfile; on Linux we implement them
//! using standard POSIX I/O and `std::fs`.

use crate::table::ShimTable;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

// ── copyfile flags ──────────────────────────────────────────────────
const COPYFILE_STAT: u32 = 1 << 1;
const COPYFILE_DATA: u32 = 1 << 3;
const COPYFILE_EXCL: u32 = 1 << 21;

// ── removefile flags ────────────────────────────────────────────────
const REMOVEFILE_RECURSIVE: u32 = 1;

// ── copyfile_state_get flags ────────────────────────────────────────
const COPYFILE_STATE_COPIED: u32 = 6;

// ── state registry ──────────────────────────────────────────────────
static NEXT_STATE_ID: AtomicU64 = AtomicU64::new(1);

struct CopyfileState {
    bytes_copied: u64,
    #[allow(dead_code)]
    src_path: Option<String>,
    #[allow(dead_code)]
    dst_path: Option<String>,
}

fn state_registry() -> &'static Mutex<HashMap<u64, CopyfileState>> {
    use std::sync::OnceLock;
    static REG: OnceLock<Mutex<HashMap<u64, CopyfileState>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

// ── helpers ─────────────────────────────────────────────────────────

unsafe fn cstr_from_guest(ptr: u64) -> Option<String> {
    if ptr == 0 {
        return None;
    }
    let c = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
    c.to_str().ok().map(|s| s.to_owned())
}

// ── shim functions ──────────────────────────────────────────────────

pub extern "C" fn shim_copyfile(from: u64, to: u64, state: u64, flags: u32) -> i32 {
    let src = match unsafe { cstr_from_guest(from) } {
        Some(s) => s,
        None => return -1,
    };
    let dst = match unsafe { cstr_from_guest(to) } {
        Some(s) => s,
        None => return -1,
    };

    // COPYFILE_EXCL — fail if destination already exists
    if flags & COPYFILE_EXCL != 0 && std::path::Path::new(&dst).exists() {
        return -1;
    }

    let mut bytes: u64 = 0;

    // COPYFILE_DATA — copy file content
    if flags & COPYFILE_DATA != 0 {
        match std::fs::copy(&src, &dst) {
            Ok(n) => bytes = n,
            Err(_) => return -1,
        }
    }

    // COPYFILE_STAT — copy permissions
    if flags & COPYFILE_STAT != 0 {
        match std::fs::metadata(&src) {
            Ok(meta) => {
                if std::fs::set_permissions(&dst, meta.permissions()).is_err() {
                    return -1;
                }
            }
            Err(_) => return -1,
        }
    }

    // Update state if provided
    if state != 0 {
        if let Ok(mut reg) = state_registry().lock() {
            if let Some(s) = reg.get_mut(&state) {
                s.bytes_copied = bytes;
                s.src_path = Some(src);
                s.dst_path = Some(dst);
            }
        }
    }

    0
}

pub extern "C" fn shim_fcopyfile(from_fd: i32, to_fd: i32, state: u64, _flags: u32) -> i32 {
    let mut buf = [0u8; 8192];
    let mut total: u64 = 0;

    loop {
        let n = unsafe { libc::read(from_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            return -1;
        }
        if n == 0 {
            break;
        }
        let mut written = 0isize;
        while written < n {
            let w = unsafe {
                libc::write(
                    to_fd,
                    buf.as_ptr().add(written as usize) as *const libc::c_void,
                    (n - written) as usize,
                )
            };
            if w <= 0 {
                return -1;
            }
            written += w;
        }
        total += n as u64;
    }

    if state != 0 {
        if let Ok(mut reg) = state_registry().lock() {
            if let Some(s) = reg.get_mut(&state) {
                s.bytes_copied = total;
            }
        }
    }

    0
}

pub extern "C" fn shim_copyfile_state_alloc() -> u64 {
    let id = NEXT_STATE_ID.fetch_add(1, Ordering::Relaxed);
    state_registry().lock().unwrap().insert(
        id,
        CopyfileState {
            bytes_copied: 0,
            src_path: None,
            dst_path: None,
        },
    );
    id
}

pub extern "C" fn shim_copyfile_state_free(state: u64) -> i32 {
    state_registry().lock().unwrap().remove(&state);
    0
}

pub extern "C" fn shim_copyfile_state_get(state: u64, flag: u32, dst: u64) -> i32 {
    if dst == 0 {
        return -1;
    }
    let reg = state_registry().lock().unwrap();
    let s = match reg.get(&state) {
        Some(s) => s,
        None => return -1,
    };
    match flag {
        COPYFILE_STATE_COPIED => {
            unsafe { *(dst as *mut u64) = s.bytes_copied };
            0
        }
        _ => -1,
    }
}

pub extern "C" fn shim_copyfile_state_set(_state: u64, _flag: u32, _src: u64) -> i32 {
    // Accept but ignore most flags.
    0
}

// ── removefile shims ────────────────────────────────────────────────

pub extern "C" fn shim_removefile(path: u64, _state: u64, flags: u32) -> i32 {
    let p = match unsafe { cstr_from_guest(path) } {
        Some(s) => s,
        None => return -1,
    };
    let path = std::path::Path::new(&p);

    if flags & REMOVEFILE_RECURSIVE != 0 && path.is_dir() {
        if std::fs::remove_dir_all(path).is_err() {
            return -1;
        }
    } else if std::fs::remove_file(path).is_err() {
        return -1;
    }

    0
}

pub extern "C" fn shim_removefile_state_alloc() -> u64 {
    NEXT_STATE_ID.fetch_add(1, Ordering::Relaxed)
}

pub extern "C" fn shim_removefile_state_free(_state: u64) -> i32 {
    0
}

// ── registration ────────────────────────────────────────────────────

pub struct CopyfileStubs;

impl CopyfileStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_copyfile", shim_copyfile as *const () as u64, "copyfile — copy files with flags"),
            ("_fcopyfile", shim_fcopyfile as *const () as u64, "fcopyfile — copy between fds"),
            ("_copyfile_state_alloc", shim_copyfile_state_alloc as *const () as u64, "copyfile_state_alloc"),
            ("_copyfile_state_free", shim_copyfile_state_free as *const () as u64, "copyfile_state_free"),
            ("_copyfile_state_get", shim_copyfile_state_get as *const () as u64, "copyfile_state_get"),
            ("_copyfile_state_set", shim_copyfile_state_set as *const () as u64, "copyfile_state_set"),
            ("_removefile", shim_removefile as *const () as u64, "removefile — remove files/dirs"),
            ("_removefile_state_alloc", shim_removefile_state_alloc as *const () as u64, "removefile_state_alloc"),
            ("_removefile_state_free", shim_removefile_state_free as *const () as u64, "removefile_state_free"),
        );
        log::debug!("CopyfileStubs: registered 9 stubs");
    }
}

// ── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_path(suffix: &str) -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "copyfile_test_{}_{}_{}",
            std::process::id(),
            id,
            suffix
        ))
    }

    fn path_to_cstring(p: &std::path::Path) -> std::ffi::CString {
        std::ffi::CString::new(p.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_copyfile_data() {
        let src = temp_path("src.txt");
        let dst = temp_path("dst.txt");
        let content = b"hello copyfile";
        std::fs::write(&src, content).unwrap();

        let c_src = path_to_cstring(&src);
        let c_dst = path_to_cstring(&dst);
        let ret = shim_copyfile(
            c_src.as_ptr() as u64,
            c_dst.as_ptr() as u64,
            0,
            COPYFILE_DATA,
        );
        assert_eq!(ret, 0);
        assert_eq!(std::fs::read(&dst).unwrap(), content);

        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn test_copyfile_excl_fails() {
        let src = temp_path("excl_src.txt");
        let dst = temp_path("excl_dst.txt");
        std::fs::write(&src, b"data").unwrap();
        std::fs::write(&dst, b"existing").unwrap();

        let c_src = path_to_cstring(&src);
        let c_dst = path_to_cstring(&dst);
        let ret = shim_copyfile(
            c_src.as_ptr() as u64,
            c_dst.as_ptr() as u64,
            0,
            COPYFILE_DATA | COPYFILE_EXCL,
        );
        assert_eq!(ret, -1);

        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn test_fcopyfile_data() {
        let src = temp_path("fc_src.bin");
        let dst = temp_path("fc_dst.bin");
        let content = b"fcopyfile test data";
        std::fs::write(&src, content).unwrap();

        let c_src = path_to_cstring(&src);
        let c_dst = path_to_cstring(&dst);

        unsafe {
            let fd_src = libc::open(c_src.as_ptr(), libc::O_RDONLY);
            assert!(fd_src >= 0);
            let fd_dst = libc::open(
                c_dst.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o644,
            );
            assert!(fd_dst >= 0);

            let ret = shim_fcopyfile(fd_src, fd_dst, 0, COPYFILE_DATA);
            assert_eq!(ret, 0);

            libc::close(fd_src);
            libc::close(fd_dst);
        }

        assert_eq!(std::fs::read(&dst).unwrap(), content);

        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn test_state_alloc_free() {
        let s = shim_copyfile_state_alloc();
        assert_ne!(s, 0);
        assert_eq!(shim_copyfile_state_free(s), 0);
    }

    #[test]
    fn test_state_get_bytes_copied() {
        let src = temp_path("st_src.txt");
        let dst = temp_path("st_dst.txt");
        let content = b"bytes counting test!!";
        std::fs::write(&src, content).unwrap();

        let state = shim_copyfile_state_alloc();

        let c_src = path_to_cstring(&src);
        let c_dst = path_to_cstring(&dst);
        let ret = shim_copyfile(
            c_src.as_ptr() as u64,
            c_dst.as_ptr() as u64,
            state,
            COPYFILE_DATA,
        );
        assert_eq!(ret, 0);

        let mut copied: u64 = 0;
        let get_ret = shim_copyfile_state_get(
            state,
            COPYFILE_STATE_COPIED,
            &mut copied as *mut u64 as u64,
        );
        assert_eq!(get_ret, 0);
        assert_eq!(copied, content.len() as u64);

        shim_copyfile_state_free(state);
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn test_removefile_basic() {
        let p = temp_path("rm_target.txt");
        std::fs::write(&p, b"delete me").unwrap();
        assert!(p.exists());

        let c_p = path_to_cstring(&p);
        let ret = shim_removefile(c_p.as_ptr() as u64, 0, 0);
        assert_eq!(ret, 0);
        assert!(!p.exists());
    }

    #[test]
    fn test_removefile_recursive() {
        let dir = temp_path("rm_dir");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("child.txt"), b"nested").unwrap();
        assert!(dir.exists());

        let c_dir = path_to_cstring(&dir);
        let ret = shim_removefile(c_dir.as_ptr() as u64, 0, REMOVEFILE_RECURSIVE);
        assert_eq!(ret, 0);
        assert!(!dir.exists());
    }
}
