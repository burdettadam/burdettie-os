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
