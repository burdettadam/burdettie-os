//! NSPipe shim: unidirectional interprocess communication channel.
//!
//! Object layout (24 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: read_fh     (u64 — NSFileHandle for reading end)
//!   offset 16: write_fh    (u64 — NSFileHandle for writing end)

use crate::nsfilehandle;
use crate::nsobj;

const READ_FH_OFF: usize = 8;
const WRITE_FH_OFF: usize = 16;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSPipe pipe]`
pub extern "C" fn shim_nspipe_pipe(_cls: u64, _cmd: u64) -> u64 {
    let mut fds = [0i32; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 { return 0; }

    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        unsafe { libc::close(fds[0]); libc::close(fds[1]); }
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nspipe_class_addr();
    }

    // Create file handles for read and write ends
    let read_fh = nsfilehandle::make_file_handle(fds[0] as i64);
    let write_fh = nsfilehandle::make_file_handle(fds[1] as i64);

    unsafe {
        *((obj as usize + READ_FH_OFF) as *mut u64) = read_fh;
        *((obj as usize + WRITE_FH_OFF) as *mut u64) = write_fh;
    }
    obj
}

// ── Instance methods ───────────────────────────────────────────

/// `-[NSPipe fileHandleForReading]`
pub extern "C" fn shim_nspipe_file_handle_for_reading(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + READ_FH_OFF) as *const u64) }
}

/// `-[NSPipe fileHandleForWriting]`
pub extern "C" fn shim_nspipe_file_handle_for_writing(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + WRITE_FH_OFF) as *const u64) }
}

pub extern "C" fn shim_nspipe_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_nsstring("<NSPipe>")
}
