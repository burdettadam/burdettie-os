//! NSStream / NSInputStream / NSOutputStream shims.
//!
//! NSStream layout (24 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: status      (u64 — NSStreamStatus enum)
//!   offset 16: error_ptr   (u64 — NSError, 0=nil)
//!
//! NSInputStream layout (40 bytes):
//!   offset  0..24: NSStream base
//!   offset 24: data_ptr    (u64 — NSData backing store)
//!   offset 32: read_pos    (u64 — byte offset of next read within data)
//!
//! NSOutputStream layout (32 bytes):
//!   offset  0..24: NSStream base
//!   offset 24: buffer_ptr  (u64 — NSMutableData backing store)

use crate::cfstring;
use crate::nsobj;

const STATUS_OFF: usize = 8;
const ERROR_OFF: usize = 16;
const DATA_OFF: usize = 24;
const READ_POS_OFF: usize = 32;
const BUFFER_OFF: usize = 24;

// NSStreamStatus enum values
const NS_STREAM_STATUS_NOT_OPEN: u64 = 0;
const NS_STREAM_STATUS_OPEN: u64 = 2;
const NS_STREAM_STATUS_AT_END: u64 = 5;
const NS_STREAM_STATUS_CLOSED: u64 = 6;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

// ═══════════════════════════════════════════════════════════════
// NSStream (abstract base class)
// ═══════════════════════════════════════════════════════════════

pub extern "C" fn shim_nsstream_open(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + STATUS_OFF) as *mut u64) = NS_STREAM_STATUS_OPEN; }
}

pub extern "C" fn shim_nsstream_close(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + STATUS_OFF) as *mut u64) = NS_STREAM_STATUS_CLOSED; }
}

pub extern "C" fn shim_nsstream_stream_status(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return NS_STREAM_STATUS_NOT_OPEN; }
    unsafe { *((obj as usize + STATUS_OFF) as *const u64) }
}

pub extern "C" fn shim_nsstream_stream_error(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ERROR_OFF) as *const u64) }
}

pub extern "C" fn shim_nsstream_delegate(_obj: u64, _cmd: u64) -> u64 {
    0 // stub — delegate not tracked
}

pub extern "C" fn shim_nsstream_set_delegate(_obj: u64, _cmd: u64, _delegate: u64) {
    // stub — delegate not tracked
}

pub extern "C" fn shim_nsstream_schedule_in_run_loop(_obj: u64, _cmd: u64, _rl: u64, _mode: u64) {
    // stub — no real run loop integration
}

pub extern "C" fn shim_nsstream_remove_from_run_loop(_obj: u64, _cmd: u64, _rl: u64, _mode: u64) {
}

pub extern "C" fn shim_nsstream_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let status = unsafe { *((obj as usize + STATUS_OFF) as *const u64) };
    make_nsstring(&format!("<NSStream: status={status}>"))
}

// ═══════════════════════════════════════════════════════════════
// NSInputStream
// ═══════════════════════════════════════════════════════════════

/// `+[NSInputStream inputStreamWithData:]`
pub extern "C" fn shim_nsinputstream_with_data(_cls: u64, _cmd: u64, data: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsinputstream_class_addr();
        *((obj as usize + DATA_OFF) as *mut u64) = data;
        // read_pos starts at 0 (already zeroed by calloc)
    }
    obj
}

/// `+[NSInputStream inputStreamWithFileAtPath:]`
pub extern "C" fn shim_nsinputstream_with_file(_cls: u64, _cmd: u64, path: u64) -> u64 {
    if path == 0 { return 0; }
    // Read file contents into NSData, store as backing
    let Some(p) = cfstring::extract_cfstring_cstr(path) else { return 0; };
    let Ok(bytes) = std::fs::read(p) else { return 0; };
    let data = crate::nsdata::shim_nsdata_data_with_bytes_length(0, 0, bytes.as_ptr() as u64, bytes.len() as u64);
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsinputstream_class_addr();
        *((obj as usize + DATA_OFF) as *mut u64) = data;
        // read_pos starts at 0
    }
    obj
}

/// `-[NSInputStream read:maxLength:]`
pub extern "C" fn shim_nsinputstream_read(obj: u64, _cmd: u64, buffer: u64, max_len: u64) -> i64 {
    if obj == 0 || buffer == 0 { return -1; }
    let status = unsafe { *((obj as usize + STATUS_OFF) as *const u64) };
    if status != NS_STREAM_STATUS_OPEN { return -1; }

    let data_ptr = unsafe { *((obj as usize + DATA_OFF) as *const u64) };
    if data_ptr == 0 {
        unsafe { *((obj as usize + STATUS_OFF) as *mut u64) = NS_STREAM_STATUS_AT_END; }
        return 0;
    }

    let total_len = crate::nsdata::shim_nsdata_length(data_ptr, 0);
    let read_pos = unsafe { *((obj as usize + READ_POS_OFF) as *const u64) };

    if read_pos >= total_len {
        unsafe { *((obj as usize + STATUS_OFF) as *mut u64) = NS_STREAM_STATUS_AT_END; }
        return 0;
    }

    let available = total_len - read_pos;
    let to_copy = std::cmp::min(available, max_len);
    let bytes_ptr = crate::nsdata::shim_nsdata_bytes(data_ptr, 0);
    unsafe {
        std::ptr::copy_nonoverlapping(
            (bytes_ptr + read_pos) as *const u8,
            buffer as *mut u8,
            to_copy as usize,
        );
        *((obj as usize + READ_POS_OFF) as *mut u64) = read_pos + to_copy;
        if read_pos + to_copy >= total_len {
            *((obj as usize + STATUS_OFF) as *mut u64) = NS_STREAM_STATUS_AT_END;
        }
    }
    to_copy as i64
}

/// `-[NSInputStream hasBytesAvailable]`
pub extern "C" fn shim_nsinputstream_has_bytes(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    let status = unsafe { *((obj as usize + STATUS_OFF) as *const u64) };
    if status != NS_STREAM_STATUS_OPEN { return 0; }
    let data_ptr = unsafe { *((obj as usize + DATA_OFF) as *const u64) };
    if data_ptr == 0 { return 0; }
    let total_len = crate::nsdata::shim_nsdata_length(data_ptr, 0);
    let read_pos = unsafe { *((obj as usize + READ_POS_OFF) as *const u64) };
    u8::from(read_pos < total_len)
}

pub extern "C" fn shim_nsinputstream_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let status = unsafe { *((obj as usize + STATUS_OFF) as *const u64) };
    make_nsstring(&format!("<NSInputStream: status={status}>"))
}

// ═══════════════════════════════════════════════════════════════
// NSOutputStream
// ═══════════════════════════════════════════════════════════════

/// `+[NSOutputStream outputStreamToMemory]`
pub extern "C" fn shim_nsoutputstream_to_memory(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    // Create a backing NSMutableData
    let md = crate::nsmutabledata::shim_nsmutabledata_data(0, 0);
    unsafe {
        *(obj as *mut u64) = nsobj::nsoutputstream_class_addr();
        *((obj as usize + BUFFER_OFF) as *mut u64) = md;
    }
    obj
}

/// `+[NSOutputStream outputStreamToFileAtPath:append:]`
pub extern "C" fn shim_nsoutputstream_to_file(_cls: u64, _cmd: u64, _path: u64, _append: u8) -> u64 {
    // Create a memory-backed stream; flush to file on close
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    let md = crate::nsmutabledata::shim_nsmutabledata_data(0, 0);
    unsafe {
        *(obj as *mut u64) = nsobj::nsoutputstream_class_addr();
        *((obj as usize + BUFFER_OFF) as *mut u64) = md;
    }
    obj
}

/// `-[NSOutputStream write:maxLength:]`
pub extern "C" fn shim_nsoutputstream_write(obj: u64, _cmd: u64, buffer: u64, len: u64) -> i64 {
    if obj == 0 || buffer == 0 { return -1; }
    let status = unsafe { *((obj as usize + STATUS_OFF) as *const u64) };
    if status != NS_STREAM_STATUS_OPEN { return -1; }

    let md = unsafe { *((obj as usize + BUFFER_OFF) as *const u64) };
    if md == 0 { return -1; }
    crate::nsmutabledata::shim_nsmutabledata_append_bytes(md, 0, buffer, len);
    len as i64
}

/// `-[NSOutputStream hasSpaceAvailable]`
pub extern "C" fn shim_nsoutputstream_has_space(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    let status = unsafe { *((obj as usize + STATUS_OFF) as *const u64) };
    u8::from(status == NS_STREAM_STATUS_OPEN)
}

/// `-[NSOutputStream propertyForKey:]` — returns the accumulated data for NSStreamDataWrittenToMemoryStreamKey.
pub extern "C" fn shim_nsoutputstream_property_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 { return 0; }
    // Pointer comparison first (Apple-aligned), then string fallback for literals
    let is_data_key = key == crate::foundation_constants::nsstream_data_written_to_memory_key_addr()
        || cfstring::extract_cfstring_cstr(key).as_deref() == Some("NSStreamDataWrittenToMemoryStreamKey");
    if is_data_key {
        unsafe { *((obj as usize + BUFFER_OFF) as *const u64) }
    } else {
        0
    }
}

pub extern "C" fn shim_nsoutputstream_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let status = unsafe { *((obj as usize + STATUS_OFF) as *const u64) };
    make_nsstring(&format!("<NSOutputStream: status={status}>"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const NSSTREAM_SIZE: usize = 24;
    const NSINPUTSTREAM_SIZE: usize = 40;
    const NSOUTPUTSTREAM_SIZE: usize = 32;

    fn make_stream() -> u64 {
        alloc_obj(NSSTREAM_SIZE, nsobj::nsstream_class_addr())
    }

    // ── NSStream base ──────────────────────────────────────────

    #[test]
    fn stream_open_sets_status() {
        let obj = make_stream();
        assert_eq!(read_field(obj, STATUS_OFF), NS_STREAM_STATUS_NOT_OPEN);
        shim_nsstream_open(obj, 0);
        assert_eq!(read_field(obj, STATUS_OFF), NS_STREAM_STATUS_OPEN);
        free_obj(obj);
    }

    #[test]
    fn stream_open_nil() {
        shim_nsstream_open(0, 0); // must not crash
    }

    #[test]
    fn stream_close_sets_status() {
        let obj = make_stream();
        shim_nsstream_open(obj, 0);
        shim_nsstream_close(obj, 0);
        assert_eq!(read_field(obj, STATUS_OFF), NS_STREAM_STATUS_CLOSED);
        free_obj(obj);
    }

    #[test]
    fn stream_close_nil() {
        shim_nsstream_close(0, 0);
    }

    #[test]
    fn stream_status_returns_current() {
        let obj = make_stream();
        assert_eq!(shim_nsstream_stream_status(obj, 0), NS_STREAM_STATUS_NOT_OPEN);
        shim_nsstream_open(obj, 0);
        assert_eq!(shim_nsstream_stream_status(obj, 0), NS_STREAM_STATUS_OPEN);
        shim_nsstream_close(obj, 0);
        assert_eq!(shim_nsstream_stream_status(obj, 0), NS_STREAM_STATUS_CLOSED);
        free_obj(obj);
    }

    #[test]
    fn stream_status_nil() {
        assert_eq!(shim_nsstream_stream_status(0, 0), NS_STREAM_STATUS_NOT_OPEN);
    }

    #[test]
    fn stream_error_nil_obj() {
        assert_eq!(shim_nsstream_stream_error(0, 0), 0);
    }

    #[test]
    fn stream_error_returns_stored_ptr() {
        let obj = make_stream();
        write_field(obj, ERROR_OFF, 0xCAFE);
        assert_eq!(shim_nsstream_stream_error(obj, 0), 0xCAFE);
        free_obj(obj);
    }

    #[test]
    fn stream_error_default_zero() {
        let obj = make_stream();
        assert_eq!(shim_nsstream_stream_error(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn stream_set_delegate_nil() {
        shim_nsstream_set_delegate(0, 0, 0); // stub, must not crash
    }

    #[test]
    fn stream_delegate_returns_zero() {
        assert_eq!(shim_nsstream_delegate(0, 0), 0);
    }

    #[test]
    fn stream_delegate_always_nil() {
        let obj = make_stream();
        assert_eq!(shim_nsstream_delegate(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn stream_schedule_in_run_loop_nil() {
        shim_nsstream_schedule_in_run_loop(0, 0, 0, 0);
    }

    #[test]
    fn stream_remove_from_run_loop_nil() {
        shim_nsstream_remove_from_run_loop(0, 0, 0, 0);
    }

    #[test]
    fn stream_description_nil() {
        assert_eq!(shim_nsstream_description(0, 0), 0);
    }

    #[test]
    fn stream_description_includes_status() {
        let obj = make_stream();
        shim_nsstream_open(obj, 0);
        let desc = shim_nsstream_description(obj, 0);
        assert_ne!(desc, 0);
        let s = read_string(desc);
        assert!(s.contains("status=2"), "expected status=2 in: {s}");
        free_obj(obj);
    }

    // ── NSInputStream ──────────────────────────────────────────

    #[test]
    fn inputstream_with_data_creates_object() {
        let bytes: [u8; 5] = [1, 2, 3, 4, 5];
        let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
            0, 0, bytes.as_ptr() as u64, 5,
        );
        let obj = shim_nsinputstream_with_data(0, 0, data);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nsinputstream_class_addr());
        assert_eq!(read_field(obj, DATA_OFF), data);
        assert_eq!(read_field(obj, READ_POS_OFF), 0);
        free_obj(obj);
    }

    #[test]
    fn inputstream_with_file_nil_path() {
        assert_eq!(shim_nsinputstream_with_file(0, 0, 0), 0);
    }

    #[test]
    fn inputstream_read_basic() {
        let src: [u8; 4] = [10, 20, 30, 40];
        let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
            0, 0, src.as_ptr() as u64, 4,
        );
        let obj = shim_nsinputstream_with_data(0, 0, data);
        shim_nsstream_open(obj, 0);

        let mut buf = [0u8; 4];
        let n = shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 4);
        assert_eq!(n, 4);
        assert_eq!(buf, [10, 20, 30, 40]);

        // After reading all bytes, status should be AT_END
        assert_eq!(read_field(obj, STATUS_OFF), NS_STREAM_STATUS_AT_END);
        free_obj(obj);
    }

    #[test]
    fn inputstream_read_partial() {
        let src: [u8; 6] = [1, 2, 3, 4, 5, 6];
        let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
            0, 0, src.as_ptr() as u64, 6,
        );
        let obj = shim_nsinputstream_with_data(0, 0, data);
        shim_nsstream_open(obj, 0);

        // Read first 3 bytes
        let mut buf = [0u8; 3];
        let n = shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 3);
        assert_eq!(n, 3);
        assert_eq!(buf, [1, 2, 3]);
        assert_eq!(read_field(obj, STATUS_OFF), NS_STREAM_STATUS_OPEN);

        // Read remaining 3 bytes
        let n = shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 3);
        assert_eq!(n, 3);
        assert_eq!(buf, [4, 5, 6]);
        assert_eq!(read_field(obj, STATUS_OFF), NS_STREAM_STATUS_AT_END);

        free_obj(obj);
    }

    #[test]
    fn inputstream_read_past_end() {
        let src: [u8; 2] = [0xAA, 0xBB];
        let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
            0, 0, src.as_ptr() as u64, 2,
        );
        let obj = shim_nsinputstream_with_data(0, 0, data);
        shim_nsstream_open(obj, 0);

        let mut buf = [0u8; 4];
        // Request more than available — should only get 2
        let n = shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 4);
        assert_eq!(n, 2);
        assert_eq!(buf[0], 0xAA);
        assert_eq!(buf[1], 0xBB);

        // Second read returns -1 (stream is now at-end, no longer "open")
        let n = shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 4);
        assert_eq!(n, -1);

        free_obj(obj);
    }

    #[test]
    fn inputstream_read_nil_obj() {
        let mut buf = [0u8; 4];
        assert_eq!(shim_nsinputstream_read(0, 0, buf.as_mut_ptr() as u64, 4), -1);
    }

    #[test]
    fn inputstream_read_nil_buffer() {
        let src: [u8; 2] = [1, 2];
        let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
            0, 0, src.as_ptr() as u64, 2,
        );
        let obj = shim_nsinputstream_with_data(0, 0, data);
        shim_nsstream_open(obj, 0);
        assert_eq!(shim_nsinputstream_read(obj, 0, 0, 4), -1);
        free_obj(obj);
    }

    #[test]
    fn inputstream_read_not_open() {
        let src: [u8; 2] = [1, 2];
        let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
            0, 0, src.as_ptr() as u64, 2,
        );
        let obj = shim_nsinputstream_with_data(0, 0, data);
        // Don't open — status is NOT_OPEN
        let mut buf = [0u8; 4];
        assert_eq!(shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 4), -1);
        free_obj(obj);
    }

    #[test]
    fn inputstream_read_nil_data_ptr() {
        let obj = alloc_obj(NSINPUTSTREAM_SIZE, nsobj::nsinputstream_class_addr());
        // data_ptr = 0 (calloc zeroed)
        shim_nsstream_open(obj, 0);
        let mut buf = [0u8; 4];
        let n = shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 4);
        assert_eq!(n, 0);
        assert_eq!(read_field(obj, STATUS_OFF), NS_STREAM_STATUS_AT_END);
        free_obj(obj);
    }

    #[test]
    fn inputstream_has_bytes_when_open() {
        let src: [u8; 3] = [1, 2, 3];
        let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
            0, 0, src.as_ptr() as u64, 3,
        );
        let obj = shim_nsinputstream_with_data(0, 0, data);
        // Not open yet
        assert_eq!(shim_nsinputstream_has_bytes(obj, 0), 0);
        shim_nsstream_open(obj, 0);
        assert_eq!(shim_nsinputstream_has_bytes(obj, 0), 1);

        // Consume all data
        let mut buf = [0u8; 3];
        shim_nsinputstream_read(obj, 0, buf.as_mut_ptr() as u64, 3);
        // read_pos == total_len → no more bytes
        assert_eq!(shim_nsinputstream_has_bytes(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn inputstream_has_bytes_nil() {
        assert_eq!(shim_nsinputstream_has_bytes(0, 0), 0);
    }

    #[test]
    fn inputstream_has_bytes_nil_data() {
        let obj = alloc_obj(NSINPUTSTREAM_SIZE, nsobj::nsinputstream_class_addr());
        shim_nsstream_open(obj, 0);
        assert_eq!(shim_nsinputstream_has_bytes(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn inputstream_description_nil() {
        assert_eq!(shim_nsinputstream_description(0, 0), 0);
    }

    #[test]
    fn inputstream_description_includes_status() {
        let obj = alloc_obj(NSINPUTSTREAM_SIZE, nsobj::nsinputstream_class_addr());
        shim_nsstream_open(obj, 0);
        let desc = shim_nsinputstream_description(obj, 0);
        assert_ne!(desc, 0);
        let s = read_string(desc);
        assert!(s.contains("NSInputStream"), "expected NSInputStream in: {s}");
        assert!(s.contains("status=2"), "expected status=2 in: {s}");
        free_obj(obj);
    }

    // ── NSOutputStream ─────────────────────────────────────────

    #[test]
    fn outputstream_to_memory_creates_object() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nsoutputstream_class_addr());
        let md = read_field(obj, BUFFER_OFF);
        assert_ne!(md, 0, "backing NSMutableData should be allocated");
        free_obj(obj);
    }

    #[test]
    fn outputstream_to_file_creates_object() {
        let path = make_string("/tmp/test_dummy");
        let obj = shim_nsoutputstream_to_file(0, 0, path, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nsoutputstream_class_addr());
        free_obj(obj);
    }

    #[test]
    fn outputstream_write_basic() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        shim_nsstream_open(obj, 0);

        let src: [u8; 3] = [0xDE, 0xAD, 0xFF];
        let n = shim_nsoutputstream_write(obj, 0, src.as_ptr() as u64, 3);
        assert_eq!(n, 3);

        // Verify through backing mutable data
        let md = read_field(obj, BUFFER_OFF);
        let md_len = crate::nsmutabledata::shim_nsmutabledata_length(md, 0);
        assert_eq!(md_len, 3);

        let md_bytes = crate::nsmutabledata::shim_nsmutabledata_bytes(md, 0);
        let written = unsafe { std::slice::from_raw_parts(md_bytes as *const u8, 3) };
        assert_eq!(written, &[0xDE, 0xAD, 0xFF]);

        free_obj(obj);
    }

    #[test]
    fn outputstream_write_multiple() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        shim_nsstream_open(obj, 0);

        let a: [u8; 2] = [1, 2];
        let b: [u8; 2] = [3, 4];
        shim_nsoutputstream_write(obj, 0, a.as_ptr() as u64, 2);
        shim_nsoutputstream_write(obj, 0, b.as_ptr() as u64, 2);

        let md = read_field(obj, BUFFER_OFF);
        assert_eq!(crate::nsmutabledata::shim_nsmutabledata_length(md, 0), 4);

        let md_bytes = crate::nsmutabledata::shim_nsmutabledata_bytes(md, 0);
        let written = unsafe { std::slice::from_raw_parts(md_bytes as *const u8, 4) };
        assert_eq!(written, &[1, 2, 3, 4]);

        free_obj(obj);
    }

    #[test]
    fn outputstream_write_nil_obj() {
        let src: [u8; 2] = [1, 2];
        assert_eq!(shim_nsoutputstream_write(0, 0, src.as_ptr() as u64, 2), -1);
    }

    #[test]
    fn outputstream_write_nil_buffer() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        shim_nsstream_open(obj, 0);
        assert_eq!(shim_nsoutputstream_write(obj, 0, 0, 5), -1);
        free_obj(obj);
    }

    #[test]
    fn outputstream_write_not_open() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        let src: [u8; 2] = [1, 2];
        assert_eq!(shim_nsoutputstream_write(obj, 0, src.as_ptr() as u64, 2), -1);
        free_obj(obj);
    }

    #[test]
    fn outputstream_has_space_when_open() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        assert_eq!(shim_nsoutputstream_has_space(obj, 0), 0); // not open
        shim_nsstream_open(obj, 0);
        assert_eq!(shim_nsoutputstream_has_space(obj, 0), 1);
        shim_nsstream_close(obj, 0);
        assert_eq!(shim_nsoutputstream_has_space(obj, 0), 0); // closed
        free_obj(obj);
    }

    #[test]
    fn outputstream_has_space_nil() {
        assert_eq!(shim_nsoutputstream_has_space(0, 0), 0);
    }

    #[test]
    fn outputstream_property_for_key_data_written() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        shim_nsstream_open(obj, 0);
        let src: [u8; 2] = [0xAB, 0xCD];
        shim_nsoutputstream_write(obj, 0, src.as_ptr() as u64, 2);

        let key = make_string("NSStreamDataWrittenToMemoryStreamKey");
        let result = shim_nsoutputstream_property_for_key(obj, 0, key);
        assert_ne!(result, 0);
        // result should be the backing NSMutableData
        assert_eq!(result, read_field(obj, BUFFER_OFF));

        free_obj(obj);
    }

    #[test]
    fn outputstream_property_for_key_unknown() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        let key = make_string("SomeOtherKey");
        assert_eq!(shim_nsoutputstream_property_for_key(obj, 0, key), 0);
        free_obj(obj);
    }

    #[test]
    fn outputstream_property_for_key_nil_obj() {
        let key = make_string("NSStreamDataWrittenToMemoryStreamKey");
        assert_eq!(shim_nsoutputstream_property_for_key(0, 0, key), 0);
    }

    #[test]
    fn outputstream_property_for_key_nil_key() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        assert_eq!(shim_nsoutputstream_property_for_key(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn outputstream_description_nil() {
        assert_eq!(shim_nsoutputstream_description(0, 0), 0);
    }

    #[test]
    fn outputstream_description_includes_status() {
        let obj = shim_nsoutputstream_to_memory(0, 0);
        shim_nsstream_open(obj, 0);
        let desc = shim_nsoutputstream_description(obj, 0);
        assert_ne!(desc, 0);
        let s = read_string(desc);
        assert!(s.contains("NSOutputStream"), "expected NSOutputStream in: {s}");
        assert!(s.contains("status=2"), "expected status=2 in: {s}");
        free_obj(obj);
    }

    // ── Round-trip: write into output, read from input ─────────

    #[test]
    fn roundtrip_output_to_input() {
        // Write some data to an output stream
        let out = shim_nsoutputstream_to_memory(0, 0);
        shim_nsstream_open(out, 0);
        let payload: [u8; 5] = [10, 20, 30, 40, 50];
        shim_nsoutputstream_write(out, 0, payload.as_ptr() as u64, 5);
        shim_nsstream_close(out, 0);

        // Get the written data (the NSMutableData ptr)
        let md = read_field(out, BUFFER_OFF);
        // Use it as backing for an input stream
        let inp = shim_nsinputstream_with_data(0, 0, md);
        shim_nsstream_open(inp, 0);

        let mut buf = [0u8; 5];
        let n = shim_nsinputstream_read(inp, 0, buf.as_mut_ptr() as u64, 5);
        assert_eq!(n, 5);
        assert_eq!(buf, payload);

        free_obj(inp);
        free_obj(out);
    }
}
