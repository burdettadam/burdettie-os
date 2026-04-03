//! NSFileHandle shim: file descriptor wrapper.
//!
//! Object layout (24 bytes):
//!   offset  0: isa  (u64)
//!   offset  8: fd   (i64 — file descriptor, -1 = invalid)
//!   offset 16: flags (u64 — 1 = close on dealloc)

use crate::cfstring;
use crate::nsobj;

const FD_OFF: usize = 8;

fn alloc_file_handle(fd: i64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsfilehandle_class_addr();
        *((obj as usize + FD_OFF) as *mut i64) = fd;
    }
    obj
}

/// Public constructor for NSPipe to create file handles for pipe fds.
pub fn make_file_handle(fd: i64) -> u64 {
    alloc_file_handle(fd)
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSFileHandle fileHandleWithStandardInput]`
pub extern "C" fn shim_nsfilehandle_stdin(_cls: u64, _cmd: u64) -> u64 {
    alloc_file_handle(0) // STDIN_FILENO
}

/// `+[NSFileHandle fileHandleWithStandardOutput]`
pub extern "C" fn shim_nsfilehandle_stdout(_cls: u64, _cmd: u64) -> u64 {
    alloc_file_handle(1) // STDOUT_FILENO
}

/// `+[NSFileHandle fileHandleWithStandardError]`
pub extern "C" fn shim_nsfilehandle_stderr(_cls: u64, _cmd: u64) -> u64 {
    alloc_file_handle(2) // STDERR_FILENO
}

/// `+[NSFileHandle fileHandleForReadingAtPath:]`
pub extern "C" fn shim_nsfilehandle_for_reading_at_path(
    _cls: u64, _cmd: u64, path: u64,
) -> u64 {
    if path == 0 { return 0; }
    let Some(p) = cfstring::extract_cfstring_cstr(path) else { return 0; };
    let c_path = std::ffi::CString::new(p).unwrap_or_default();
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
    if fd < 0 { return 0; }
    alloc_file_handle(fd as i64)
}

/// `+[NSFileHandle fileHandleForWritingAtPath:]`
pub extern "C" fn shim_nsfilehandle_for_writing_at_path(
    _cls: u64, _cmd: u64, path: u64,
) -> u64 {
    if path == 0 { return 0; }
    let Some(p) = cfstring::extract_cfstring_cstr(path) else { return 0; };
    let c_path = std::ffi::CString::new(p).unwrap_or_default();
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_WRONLY | libc::O_CREAT, 0o644) };
    if fd < 0 { return 0; }
    alloc_file_handle(fd as i64)
}

/// `+[NSFileHandle fileHandleForUpdatingAtPath:]` — open for read+write.
pub extern "C" fn shim_nsfilehandle_for_updating_at_path(
    _cls: u64, _cmd: u64, path: u64,
) -> u64 {
    if path == 0 { return 0; }
    let Some(p) = cfstring::extract_cfstring_cstr(path) else { return 0; };
    let c_path = std::ffi::CString::new(p).unwrap_or_default();
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR) };
    if fd < 0 { return 0; }
    alloc_file_handle(fd as i64)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSFileHandle fileDescriptor]`
pub extern "C" fn shim_nsfilehandle_file_descriptor(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return -1; }
    unsafe { *((obj as usize + FD_OFF) as *const i64) }
}

/// `-[NSFileHandle readDataOfLength:]`
pub extern "C" fn shim_nsfilehandle_read_data_of_length(
    obj: u64, _cmd: u64, length: u64,
) -> u64 {
    if obj == 0 || length == 0 { return crate::nsdata::make_nsdata(&[]); }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    let mut buf = vec![0u8; length as usize];
    let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), length as usize) };
    if n <= 0 { return crate::nsdata::make_nsdata(&[]); }
    crate::nsdata::make_nsdata(&buf[..n as usize])
}

/// `-[NSFileHandle readDataToEndOfFile]`
pub extern "C" fn shim_nsfilehandle_read_data_to_end(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return crate::nsdata::make_nsdata(&[]); }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    let mut result = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 { break; }
        result.extend_from_slice(&buf[..n as usize]);
    }
    crate::nsdata::make_nsdata(&result)
}

/// `-[NSFileHandle availableData]`
pub extern "C" fn shim_nsfilehandle_available_data(obj: u64, _cmd: u64) -> u64 {
    shim_nsfilehandle_read_data_to_end(obj, _cmd)
}

/// `-[NSFileHandle writeData:]`
pub extern "C" fn shim_nsfilehandle_write_data(obj: u64, _cmd: u64, data: u64) -> u64 {
    if obj == 0 || data == 0 { return 0; }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    let len = crate::nsdata::shim_nsdata_length(data, 0);
    let bytes = crate::nsdata::shim_nsdata_bytes(data, 0);
    if len > 0 && bytes != 0 {
        unsafe { libc::write(fd, bytes as *const libc::c_void, len as usize); }
    }
    0
}

/// `-[NSFileHandle closeFile]`
pub extern "C" fn shim_nsfilehandle_close_file(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    if fd > 2 { // Don't close stdin/stdout/stderr
        unsafe { libc::close(fd); }
        unsafe { *((obj as usize + FD_OFF) as *mut i64) = -1; }
    }
    0
}

/// `-[NSFileHandle seekToFileOffset:]`
pub extern "C" fn shim_nsfilehandle_seek_to_file_offset(
    obj: u64, _cmd: u64, offset: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    unsafe { libc::lseek(fd, offset as libc::off_t, libc::SEEK_SET); }
    0
}

/// `-[NSFileHandle seekToEndOfFile]` — seeks to EOF, returns new offset.
pub extern "C" fn shim_nsfilehandle_seek_to_end_of_file(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    let pos = unsafe { libc::lseek(fd, 0, libc::SEEK_END) };
    if pos < 0 { 0 } else { pos as u64 }
}

/// `-[NSFileHandle truncateFileAtOffset:]` — truncates file to given offset.
pub extern "C" fn shim_nsfilehandle_truncate_at_offset(
    obj: u64, _cmd: u64, offset: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    unsafe { libc::ftruncate(fd, offset as libc::off_t); }
    0
}

/// `-[NSFileHandle offsetInFile]`
pub extern "C" fn shim_nsfilehandle_offset_in_file(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) } as i32;
    let pos = unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) };
    if pos < 0 { 0 } else { pos as u64 }
}

/// `-[NSFileHandle description]`
pub extern "C" fn shim_nsfilehandle_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let fd = unsafe { *((obj as usize + FD_OFF) as *const i64) };
    let desc = format!("<NSFileHandle: fd={fd}>");
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn isa() -> u64 {
        crate::nsobj::nsfilehandle_class_addr()
    }

    /// Helper: create a libc pipe, returns (read_fd, write_fd).
    fn make_pipe() -> (i32, i32) {
        let mut fds = [0i32; 2];
        let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(ret, 0, "pipe() failed");
        (fds[0], fds[1])
    }

    /// Helper: create a temp file with `data`, return its path.
    fn make_temp_file(data: &[u8]) -> String {
        use std::io::Write;
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = format!("/tmp/_nsfh_test_{}_{}", std::process::id(), id);
        let mut f = std::fs::File::create(&path).expect("create temp file");
        f.write_all(data).expect("write temp file");
        f.flush().expect("flush temp file");
        drop(f);
        path
    }

    // ── make_file_handle ────────────────────────────────────────

    #[test]
    fn test_make_file_handle_basic() {
        let obj = make_file_handle(42);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), isa());
        assert_eq!(read_field_i64(obj, FD_OFF), 42);
        free_obj(obj);
    }

    #[test]
    fn test_make_file_handle_negative_fd() {
        let obj = make_file_handle(-1);
        assert_ne!(obj, 0);
        assert_eq!(read_field_i64(obj, FD_OFF), -1);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_stdin / stdout / stderr ───────────────

    #[test]
    fn test_stdin() {
        let obj = shim_nsfilehandle_stdin(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field_i64(obj, FD_OFF), 0);
        free_obj(obj);
    }

    #[test]
    fn test_stdout() {
        let obj = shim_nsfilehandle_stdout(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field_i64(obj, FD_OFF), 1);
        free_obj(obj);
    }

    #[test]
    fn test_stderr() {
        let obj = shim_nsfilehandle_stderr(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field_i64(obj, FD_OFF), 2);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_for_reading_at_path ───────────────────

    #[test]
    fn test_for_reading_at_path_nil() {
        assert_eq!(shim_nsfilehandle_for_reading_at_path(0, 0, 0), 0);
    }

    #[test]
    fn test_for_reading_at_path_basic() {
        let path_str = make_temp_file(b"hello");
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_reading_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);
        let fd = read_field_i64(obj, FD_OFF);
        assert!(fd >= 0, "fd should be non-negative");
        // Clean up
        shim_nsfilehandle_close_file(obj, 0);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }

    #[test]
    fn test_for_reading_at_path_nonexistent() {
        let ns_path = make_string("/tmp/__nsfilehandle_nonexistent_test_file__");
        let obj = shim_nsfilehandle_for_reading_at_path(0, 0, ns_path);
        assert_eq!(obj, 0);
        free_obj(ns_path);
    }

    // ── shim_nsfilehandle_for_writing_at_path ───────────────────

    #[test]
    fn test_for_writing_at_path_nil() {
        assert_eq!(shim_nsfilehandle_for_writing_at_path(0, 0, 0), 0);
    }

    #[test]
    fn test_for_writing_at_path_basic() {
        let path_str = format!("/tmp/__nsfilehandle_write_test_{}", std::process::id());
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_writing_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);
        let fd = read_field_i64(obj, FD_OFF);
        assert!(fd >= 0);
        shim_nsfilehandle_close_file(obj, 0);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }

    // ── shim_nsfilehandle_for_updating_at_path ──────────────────

    #[test]
    fn test_for_updating_at_path_nil() {
        assert_eq!(shim_nsfilehandle_for_updating_at_path(0, 0, 0), 0);
    }

    #[test]
    fn test_for_updating_at_path_basic() {
        let path_str = make_temp_file(b"update me");
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_updating_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);
        let fd = read_field_i64(obj, FD_OFF);
        assert!(fd >= 0);
        shim_nsfilehandle_close_file(obj, 0);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }

    #[test]
    fn test_for_updating_at_path_nonexistent() {
        let ns_path = make_string("/tmp/__nsfilehandle_update_nonexistent__");
        let obj = shim_nsfilehandle_for_updating_at_path(0, 0, ns_path);
        // O_RDWR without O_CREAT on nonexistent file should fail
        assert_eq!(obj, 0);
        free_obj(ns_path);
    }

    // ── shim_nsfilehandle_file_descriptor ────────────────────────

    #[test]
    fn test_file_descriptor_nil() {
        assert_eq!(shim_nsfilehandle_file_descriptor(0, 0), -1);
    }

    #[test]
    fn test_file_descriptor_basic() {
        let obj = make_file_handle(99);
        assert_eq!(shim_nsfilehandle_file_descriptor(obj, 0), 99);
        free_obj(obj);
    }

    #[test]
    fn test_file_descriptor_negative() {
        let obj = make_file_handle(-1);
        assert_eq!(shim_nsfilehandle_file_descriptor(obj, 0), -1);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_read_data_of_length ───────────────────

    #[test]
    fn test_read_data_of_length_nil() {
        let data = shim_nsfilehandle_read_data_of_length(0, 0, 10);
        // Returns empty NSData, not 0
        assert_ne!(data, 0);
        assert_eq!(crate::nsdata::shim_nsdata_length(data, 0), 0);
        free_obj(data);
    }

    #[test]
    fn test_read_data_of_length_zero_length() {
        let obj = make_file_handle(0);
        let data = shim_nsfilehandle_read_data_of_length(obj, 0, 0);
        assert_ne!(data, 0);
        assert_eq!(crate::nsdata::shim_nsdata_length(data, 0), 0);
        free_obj(data);
        free_obj(obj);
    }

    #[test]
    fn test_read_data_of_length_from_pipe() {
        let (rfd, wfd) = make_pipe();
        let msg = b"pipe data";
        unsafe { libc::write(wfd, msg.as_ptr().cast(), msg.len()); }
        unsafe { libc::close(wfd); }

        let obj = make_file_handle(rfd as i64);
        let data = shim_nsfilehandle_read_data_of_length(obj, 0, 100);
        assert_ne!(data, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert_eq!(len, msg.len() as u64);

        let bytes_ptr = crate::nsdata::shim_nsdata_bytes(data, 0);
        let slice = unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, len as usize) };
        assert_eq!(slice, msg);

        unsafe { libc::close(rfd); }
        free_obj(data);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_read_data_to_end ──────────────────────

    #[test]
    fn test_read_data_to_end_nil() {
        let data = shim_nsfilehandle_read_data_to_end(0, 0);
        assert_ne!(data, 0);
        assert_eq!(crate::nsdata::shim_nsdata_length(data, 0), 0);
        free_obj(data);
    }

    #[test]
    fn test_read_data_to_end_from_pipe() {
        let (rfd, wfd) = make_pipe();
        let msg = b"all the data";
        unsafe { libc::write(wfd, msg.as_ptr().cast(), msg.len()); }
        unsafe { libc::close(wfd); }

        let obj = make_file_handle(rfd as i64);
        let data = shim_nsfilehandle_read_data_to_end(obj, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert_eq!(len, msg.len() as u64);

        unsafe { libc::close(rfd); }
        free_obj(data);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_available_data ─────────────────────────

    #[test]
    fn test_available_data_nil() {
        let data = shim_nsfilehandle_available_data(0, 0);
        assert_ne!(data, 0);
        assert_eq!(crate::nsdata::shim_nsdata_length(data, 0), 0);
        free_obj(data);
    }

    #[test]
    fn test_available_data_from_pipe() {
        let (rfd, wfd) = make_pipe();
        let msg = b"avail";
        unsafe { libc::write(wfd, msg.as_ptr().cast(), msg.len()); }
        unsafe { libc::close(wfd); }

        let obj = make_file_handle(rfd as i64);
        let data = shim_nsfilehandle_available_data(obj, 0);
        assert_eq!(crate::nsdata::shim_nsdata_length(data, 0), msg.len() as u64);

        unsafe { libc::close(rfd); }
        free_obj(data);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_write_data ────────────────────────────

    #[test]
    fn test_write_data_nil_obj() {
        assert_eq!(shim_nsfilehandle_write_data(0, 0, 1), 0);
    }

    #[test]
    fn test_write_data_nil_data() {
        let obj = make_file_handle(1);
        assert_eq!(shim_nsfilehandle_write_data(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_write_data_roundtrip() {
        let (rfd, wfd) = make_pipe();
        let obj = make_file_handle(wfd as i64);

        let payload = b"written";
        let ns_data = crate::nsdata::make_nsdata(payload);
        shim_nsfilehandle_write_data(obj, 0, ns_data);

        // Close the write end so we can read all from the read end
        unsafe { libc::close(wfd); }

        let mut buf = [0u8; 64];
        let n = unsafe { libc::read(rfd, buf.as_mut_ptr().cast(), buf.len()) };
        assert_eq!(n as usize, payload.len());
        assert_eq!(&buf[..n as usize], payload);

        unsafe { libc::close(rfd); }
        free_obj(ns_data);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_close_file ────────────────────────────

    #[test]
    fn test_close_file_nil() {
        assert_eq!(shim_nsfilehandle_close_file(0, 0), 0);
    }

    #[test]
    fn test_close_file_closes_fd() {
        let (rfd, wfd) = make_pipe();
        unsafe { libc::close(wfd); }

        let obj = make_file_handle(rfd as i64);
        shim_nsfilehandle_close_file(obj, 0);
        // fd should now be -1 in the object
        assert_eq!(read_field_i64(obj, FD_OFF), -1);
        free_obj(obj);
    }

    #[test]
    fn test_close_file_does_not_close_stdio() {
        // fd=0 (stdin) should NOT be closed
        let obj = make_file_handle(0);
        shim_nsfilehandle_close_file(obj, 0);
        // fd should still be 0 (not -1)
        assert_eq!(read_field_i64(obj, FD_OFF), 0);
        free_obj(obj);
    }

    // ── shim_nsfilehandle_seek_to_file_offset ───────────────────

    #[test]
    fn test_seek_to_file_offset_nil() {
        assert_eq!(shim_nsfilehandle_seek_to_file_offset(0, 0, 10), 0);
    }

    #[test]
    fn test_seek_to_file_offset_basic() {
        let path_str = make_temp_file(b"0123456789");
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_reading_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);

        shim_nsfilehandle_seek_to_file_offset(obj, 0, 5);
        let off = shim_nsfilehandle_offset_in_file(obj, 0);
        assert_eq!(off, 5);

        shim_nsfilehandle_close_file(obj, 0);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }

    // ── shim_nsfilehandle_seek_to_end_of_file ───────────────────

    #[test]
    fn test_seek_to_end_of_file_nil() {
        assert_eq!(shim_nsfilehandle_seek_to_end_of_file(0, 0), 0);
    }

    #[test]
    fn test_seek_to_end_of_file_basic() {
        let path_str = make_temp_file(b"abcdefghij");
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_reading_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);

        let end_pos = shim_nsfilehandle_seek_to_end_of_file(obj, 0);
        assert_eq!(end_pos, 10);

        shim_nsfilehandle_close_file(obj, 0);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }

    // ── shim_nsfilehandle_truncate_at_offset ────────────────────

    #[test]
    fn test_truncate_at_offset_nil() {
        assert_eq!(shim_nsfilehandle_truncate_at_offset(0, 0, 0), 0);
    }

    #[test]
    fn test_truncate_at_offset_basic() {
        let path_str = make_temp_file(b"0123456789");
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_updating_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);

        shim_nsfilehandle_truncate_at_offset(obj, 0, 5);

        // Verify file is now 5 bytes by seeking to end
        let end_pos = shim_nsfilehandle_seek_to_end_of_file(obj, 0);
        assert_eq!(end_pos, 5);

        shim_nsfilehandle_close_file(obj, 0);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }

    // ── shim_nsfilehandle_offset_in_file ────────────────────────

    #[test]
    fn test_offset_in_file_nil() {
        assert_eq!(shim_nsfilehandle_offset_in_file(0, 0), 0);
    }

    #[test]
    fn test_offset_in_file_at_start() {
        let path_str = make_temp_file(b"test");
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_reading_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);

        assert_eq!(shim_nsfilehandle_offset_in_file(obj, 0), 0);

        shim_nsfilehandle_close_file(obj, 0);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }

    // ── shim_nsfilehandle_description ───────────────────────────

    #[test]
    fn test_description_nil() {
        assert_eq!(shim_nsfilehandle_description(0, 0), 0);
    }

    #[test]
    fn test_description_basic() {
        let obj = make_file_handle(7);
        let desc = shim_nsfilehandle_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSFileHandle: fd=7>");
        free_obj(desc);
        free_obj(obj);
    }

    #[test]
    fn test_description_negative_fd() {
        let obj = make_file_handle(-1);
        let desc = shim_nsfilehandle_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSFileHandle: fd=-1>");
        free_obj(desc);
        free_obj(obj);
    }

    // ── read + seek integration ─────────────────────────────────

    #[test]
    fn test_seek_then_read() {
        let path_str = make_temp_file(b"abcdefghij");
        let ns_path = make_string(&path_str);
        let obj = shim_nsfilehandle_for_reading_at_path(0, 0, ns_path);
        assert_ne!(obj, 0);

        // Seek to offset 5, read 5 bytes
        shim_nsfilehandle_seek_to_file_offset(obj, 0, 5);
        let data = shim_nsfilehandle_read_data_of_length(obj, 0, 5);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert_eq!(len, 5);
        let bytes_ptr = crate::nsdata::shim_nsdata_bytes(data, 0);
        let slice = unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, len as usize) };
        assert_eq!(slice, b"fghij");

        shim_nsfilehandle_close_file(obj, 0);
        free_obj(data);
        free_obj(obj);
        free_obj(ns_path);
        std::fs::remove_file(&path_str).ok();
    }
}
