//! NSMutableData shim: mutable byte buffer.
//!
//! Object layout (32 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: length       (u64 — current byte count)
//!   offset 16: bytes_ptr    (u64 — pointer to malloc'd byte buffer)
//!   offset 24: capacity     (u64 — allocated buffer size)

const LENGTH_OFF: usize = 8;
const BYTES_OFF: usize = 16;
const CAP_OFF: usize = 24;
const INITIAL_CAP: u64 = 16;

fn alloc_mutable_data(cap: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmutabledata_class_addr();
    }
    let real_cap = if cap == 0 { INITIAL_CAP } else { cap };
    let buf = crate::pal_alloc().calloc(real_cap as usize, 1) as u64;
    if buf != 0 {
        unsafe {
            *((obj as usize + BYTES_OFF) as *mut u64) = buf;
        }
    }
    unsafe {
        *((obj as usize + CAP_OFF) as *mut u64) = real_cap;
    }
    obj
}

unsafe fn grow_if_needed(obj: u64, needed: u64) {
    let len = *((obj as usize + LENGTH_OFF) as *const u64);
    let cap = *((obj as usize + CAP_OFF) as *const u64);
    let required = len + needed;
    if required <= cap {
        return;
    }
    let mut new_cap = if cap == 0 { INITIAL_CAP } else { cap };
    while new_cap < required {
        new_cap *= 2;
    }
    let old_buf = *((obj as usize + BYTES_OFF) as *const u64);
    let new_buf = crate::pal_alloc().realloc(old_buf as *mut u8, new_cap as usize) as u64;
    if new_buf != 0 {
        // Zero new region
        let start = (new_buf as usize + cap as usize) as *mut u8;
        std::ptr::write_bytes(start, 0, (new_cap - cap) as usize);
        *((obj as usize + BYTES_OFF) as *mut u64) = new_buf;
        *((obj as usize + CAP_OFF) as *mut u64) = new_cap;
    }
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSMutableData data]`
pub extern "C" fn shim_nsmutabledata_data(_cls: u64, _cmd: u64) -> u64 {
    alloc_mutable_data(INITIAL_CAP)
}

/// `+[NSMutableData dataWithCapacity:]`
pub extern "C" fn shim_nsmutabledata_data_with_capacity(_cls: u64, _cmd: u64, cap: u64) -> u64 {
    alloc_mutable_data(cap)
}

/// `+[NSMutableData dataWithLength:]`
pub extern "C" fn shim_nsmutabledata_data_with_length(_cls: u64, _cmd: u64, length: u64) -> u64 {
    let obj = alloc_mutable_data(length);
    if obj != 0 {
        unsafe {
            *((obj as usize + LENGTH_OFF) as *mut u64) = length;
        }
    }
    obj
}

/// `+[NSMutableData dataWithBytes:length:]`
pub extern "C" fn shim_nsmutabledata_data_with_bytes(
    _cls: u64,
    _cmd: u64,
    bytes: u64,
    length: u64,
) -> u64 {
    let obj = alloc_mutable_data(length);
    if obj == 0 || length == 0 || bytes == 0 {
        return obj;
    }
    let buf = unsafe { *((obj as usize + BYTES_OFF) as *const u64) };
    if buf != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes as *const u8, buf as *mut u8, length as usize);
        }
    }
    unsafe {
        *((obj as usize + LENGTH_OFF) as *mut u64) = length;
    }
    obj
}

// ── Instance methods (inherited from NSData) ───────────────────────

/// `-[NSMutableData length]`
pub extern "C" fn shim_nsmutabledata_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + LENGTH_OFF) as *const u64) }
}

/// `-[NSMutableData bytes]` / `-[NSMutableData mutableBytes]`
pub extern "C" fn shim_nsmutabledata_bytes(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + BYTES_OFF) as *const u64) }
}

// ── Mutation methods ───────────────────────────────────────────────

/// `-[NSMutableData appendBytes:length:]`
pub extern "C" fn shim_nsmutabledata_append_bytes(
    obj: u64,
    _cmd: u64,
    bytes: u64,
    length: u64,
) -> u64 {
    if obj == 0 || length == 0 || bytes == 0 {
        return 0;
    }
    unsafe {
        grow_if_needed(obj, length);
    }
    let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    let buf = unsafe { *((obj as usize + BYTES_OFF) as *const u64) };
    if buf != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes as *const u8,
                (buf as usize + len as usize) as *mut u8,
                length as usize,
            );
            *((obj as usize + LENGTH_OFF) as *mut u64) = len + length;
        }
    }
    0
}

/// `-[NSMutableData appendData:]`
pub extern "C" fn shim_nsmutabledata_append_data(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 || other == 0 {
        return 0;
    }
    let other_len = crate::nsdata::shim_nsdata_length(other, 0);
    let other_bytes = crate::nsdata::shim_nsdata_bytes(other, 0);
    if other_len > 0 && other_bytes != 0 {
        shim_nsmutabledata_append_bytes(obj, 0, other_bytes, other_len);
    }
    0
}

/// `-[NSMutableData setLength:]`
pub extern "C" fn shim_nsmutabledata_set_length(obj: u64, _cmd: u64, new_len: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let old_len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    if new_len > old_len {
        unsafe {
            grow_if_needed(obj, new_len - old_len);
        }
    }
    unsafe {
        *((obj as usize + LENGTH_OFF) as *mut u64) = new_len;
    }
    0
}

/// `-[NSMutableData resetBytesInRange:]` (loc, len)
pub extern "C" fn shim_nsmutabledata_reset_bytes_in_range(
    obj: u64,
    _cmd: u64,
    loc: u64,
    len: u64,
) -> u64 {
    if obj == 0 || len == 0 {
        return 0;
    }
    let buf = unsafe { *((obj as usize + BYTES_OFF) as *const u64) };
    let data_len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    if buf == 0 {
        return 0;
    }
    let end = (loc + len).min(data_len);
    if loc < end {
        unsafe {
            std::ptr::write_bytes(
                (buf as usize + loc as usize) as *mut u8,
                0,
                (end - loc) as usize,
            );
        }
    }
    0
}

/// `-[NSMutableData replaceBytesInRange:withBytes:]` (loc, len, bytes)
pub extern "C" fn shim_nsmutabledata_replace_bytes_in_range(
    obj: u64,
    _cmd: u64,
    loc: u64,
    len: u64,
    bytes: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let buf = unsafe { *((obj as usize + BYTES_OFF) as *const u64) };
    let data_len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    if buf == 0 {
        return 0;
    }
    let end = (loc + len).min(data_len);
    if bytes != 0 && loc < end {
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes as *const u8,
                (buf as usize + loc as usize) as *mut u8,
                (end - loc) as usize,
            );
        }
    }
    0
}

/// `-[NSMutableData description]`
pub extern "C" fn shim_nsmutabledata_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    let desc = format!("<NSMutableData: {} bytes>", len);
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // Helper: read raw bytes from an NSMutableData's buffer.
    unsafe fn read_bytes(obj: u64, count: usize) -> Vec<u8> {
        let buf = *((obj as usize + BYTES_OFF) as *const u64);
        if buf == 0 || count == 0 {
            return vec![];
        }
        let slice = std::slice::from_raw_parts(buf as *const u8, count);
        slice.to_vec()
    }

    // Helper: free an NSMutableData (frees both the internal buffer and the obj).
    unsafe fn free_md(obj: u64) {
        if obj == 0 {
            return;
        }
        let buf = *((obj as usize + BYTES_OFF) as *const u64);
        if buf != 0 {
            crate::pal_alloc().free(buf as *mut u8);
        }
        free_obj(obj);
    }

    // ── +[NSMutableData data] ──────────────────────────────────────

    #[test]
    fn data_returns_valid_object() {
        let obj = shim_nsmutabledata_data(0, 0);
        assert_ne!(obj, 0);
        let len = shim_nsmutabledata_length(obj, 0);
        assert_eq!(len, 0);
        unsafe {
            free_md(obj);
        }
    }

    // ── +[NSMutableData dataWithCapacity:] ─────────────────────────

    #[test]
    fn data_with_capacity_basic() {
        let obj = shim_nsmutabledata_data_with_capacity(0, 0, 256);
        assert_ne!(obj, 0);
        let len = shim_nsmutabledata_length(obj, 0);
        assert_eq!(len, 0);
        // capacity stored internally
        let cap = read_field(obj, CAP_OFF);
        assert_eq!(cap, 256);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn data_with_capacity_zero_gets_default() {
        let obj = shim_nsmutabledata_data_with_capacity(0, 0, 0);
        assert_ne!(obj, 0);
        let cap = read_field(obj, CAP_OFF);
        assert_eq!(cap, INITIAL_CAP);
        unsafe {
            free_md(obj);
        }
    }

    // ── +[NSMutableData dataWithLength:] ───────────────────────────

    #[test]
    fn data_with_length_sets_length() {
        let obj = shim_nsmutabledata_data_with_length(0, 0, 64);
        assert_ne!(obj, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 64);
        // buffer should be zeroed
        let bytes = unsafe { read_bytes(obj, 64) };
        assert!(bytes.iter().all(|&b| b == 0));
        unsafe {
            free_md(obj);
        }
    }

    // ── +[NSMutableData dataWithBytes:length:] ─────────────────────

    #[test]
    fn data_with_bytes_copies_data() {
        let src: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBE];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 4);
        assert_ne!(obj, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 4);
        let bytes = unsafe { read_bytes(obj, 4) };
        assert_eq!(bytes, vec![0xCA, 0xFE, 0xBA, 0xBE]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn data_with_bytes_zero_length() {
        let src: [u8; 1] = [0xFF];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 0);
        assert_ne!(obj, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn data_with_bytes_null_ptr() {
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, 0, 10);
        assert_ne!(obj, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);
        unsafe {
            free_md(obj);
        }
    }

    // ── -[NSMutableData length] / bytes ────────────────────────────

    #[test]
    fn length_nil_returns_zero() {
        assert_eq!(shim_nsmutabledata_length(0, 0), 0);
    }

    #[test]
    fn bytes_nil_returns_zero() {
        assert_eq!(shim_nsmutabledata_bytes(0, 0), 0);
    }

    #[test]
    fn bytes_returns_buffer_ptr() {
        let obj = shim_nsmutabledata_data(0, 0);
        let ptr = shim_nsmutabledata_bytes(obj, 0);
        assert_ne!(ptr, 0);
        unsafe {
            free_md(obj);
        }
    }

    // ── -[NSMutableData appendBytes:length:] ───────────────────────

    #[test]
    fn append_bytes_basic() {
        let obj = shim_nsmutabledata_data(0, 0);
        let src: [u8; 3] = [1, 2, 3];
        shim_nsmutabledata_append_bytes(obj, 0, src.as_ptr() as u64, 3);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 3);
        let bytes = unsafe { read_bytes(obj, 3) };
        assert_eq!(bytes, vec![1, 2, 3]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn append_bytes_multiple_times() {
        let obj = shim_nsmutabledata_data(0, 0);
        let a: [u8; 2] = [0xAA, 0xBB];
        let b: [u8; 2] = [0xCC, 0xDD];
        shim_nsmutabledata_append_bytes(obj, 0, a.as_ptr() as u64, 2);
        shim_nsmutabledata_append_bytes(obj, 0, b.as_ptr() as u64, 2);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 4);
        let bytes = unsafe { read_bytes(obj, 4) };
        assert_eq!(bytes, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn append_bytes_triggers_growth() {
        let obj = shim_nsmutabledata_data_with_capacity(0, 0, 4);
        let src: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        shim_nsmutabledata_append_bytes(obj, 0, src.as_ptr() as u64, 8);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 8);
        let bytes = unsafe { read_bytes(obj, 8) };
        assert_eq!(bytes, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn append_bytes_nil_obj() {
        let src: [u8; 2] = [1, 2];
        // Must not crash
        let r = shim_nsmutabledata_append_bytes(0, 0, src.as_ptr() as u64, 2);
        assert_eq!(r, 0);
    }

    #[test]
    fn append_bytes_zero_length() {
        let obj = shim_nsmutabledata_data(0, 0);
        let src: [u8; 2] = [1, 2];
        shim_nsmutabledata_append_bytes(obj, 0, src.as_ptr() as u64, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn append_bytes_null_ptr() {
        let obj = shim_nsmutabledata_data(0, 0);
        shim_nsmutabledata_append_bytes(obj, 0, 0, 5);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);
        unsafe {
            free_md(obj);
        }
    }

    // ── -[NSMutableData appendData:] ───────────────────────────────

    #[test]
    fn append_data_from_nsdata() {
        let obj = shim_nsmutabledata_data(0, 0);
        let src = crate::nsdata::make_nsdata(&[10, 20, 30]);
        shim_nsmutabledata_append_data(obj, 0, src);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 3);
        let bytes = unsafe { read_bytes(obj, 3) };
        assert_eq!(bytes, vec![10, 20, 30]);
        // Cross-check with nsdata shim
        assert_eq!(crate::nsdata::shim_nsdata_length(src, 0), 3);
        free_obj(src); // nsdata needs separate free
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn append_data_nil_other() {
        let obj = shim_nsmutabledata_data(0, 0);
        let r = shim_nsmutabledata_append_data(obj, 0, 0);
        assert_eq!(r, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn append_data_nil_obj() {
        let r = shim_nsmutabledata_append_data(0, 0, 0);
        assert_eq!(r, 0);
    }

    #[test]
    fn append_data_empty_nsdata() {
        let obj = shim_nsmutabledata_data(0, 0);
        let empty = crate::nsdata::make_nsdata(&[]);
        shim_nsmutabledata_append_data(obj, 0, empty);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);
        free_obj(empty);
        unsafe {
            free_md(obj);
        }
    }

    // ── -[NSMutableData setLength:] ────────────────────────────────

    #[test]
    fn set_length_grow() {
        let obj = shim_nsmutabledata_data(0, 0);
        shim_nsmutabledata_set_length(obj, 0, 100);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 100);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn set_length_shrink() {
        let src: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 8);
        shim_nsmutabledata_set_length(obj, 0, 3);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 3);
        // Original bytes should still be intact
        let bytes = unsafe { read_bytes(obj, 3) };
        assert_eq!(bytes, vec![1, 2, 3]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn set_length_zero() {
        let obj = shim_nsmutabledata_data_with_length(0, 0, 50);
        shim_nsmutabledata_set_length(obj, 0, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn set_length_nil_obj() {
        let r = shim_nsmutabledata_set_length(0, 0, 100);
        assert_eq!(r, 0);
    }

    // ── -[NSMutableData resetBytesInRange:] ────────────────────────

    #[test]
    fn reset_bytes_in_range_basic() {
        let src: [u8; 6] = [1, 2, 3, 4, 5, 6];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 6);
        // Zero out bytes at indices 2..4 (loc=2, len=2)
        shim_nsmutabledata_reset_bytes_in_range(obj, 0, 2, 2);
        let bytes = unsafe { read_bytes(obj, 6) };
        assert_eq!(bytes, vec![1, 2, 0, 0, 5, 6]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn reset_bytes_in_range_clamps_to_length() {
        let src: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 4);
        // range extends past length: loc=2, len=100 — should clamp to [2..4)
        shim_nsmutabledata_reset_bytes_in_range(obj, 0, 2, 100);
        let bytes = unsafe { read_bytes(obj, 4) };
        assert_eq!(bytes, vec![0xFF, 0xFF, 0, 0]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn reset_bytes_in_range_zero_length() {
        let src: [u8; 3] = [1, 2, 3];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 3);
        shim_nsmutabledata_reset_bytes_in_range(obj, 0, 0, 0);
        let bytes = unsafe { read_bytes(obj, 3) };
        assert_eq!(bytes, vec![1, 2, 3]); // unchanged
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn reset_bytes_in_range_nil_obj() {
        let r = shim_nsmutabledata_reset_bytes_in_range(0, 0, 0, 5);
        assert_eq!(r, 0);
    }

    // ── -[NSMutableData replaceBytesInRange:withBytes:] ────────────

    #[test]
    fn replace_bytes_basic() {
        let src: [u8; 5] = [1, 2, 3, 4, 5];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 5);
        let replacement: [u8; 2] = [0xAA, 0xBB];
        shim_nsmutabledata_replace_bytes_in_range(obj, 0, 1, 2, replacement.as_ptr() as u64);
        let bytes = unsafe { read_bytes(obj, 5) };
        assert_eq!(bytes, vec![1, 0xAA, 0xBB, 4, 5]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn replace_bytes_clamped() {
        let src: [u8; 4] = [1, 2, 3, 4];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 4);
        let replacement: [u8; 10] = [0xFF; 10];
        // Range extends past data length — should clamp
        shim_nsmutabledata_replace_bytes_in_range(obj, 0, 3, 10, replacement.as_ptr() as u64);
        let bytes = unsafe { read_bytes(obj, 4) };
        assert_eq!(bytes, vec![1, 2, 3, 0xFF]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn replace_bytes_null_bytes_ptr() {
        let src: [u8; 3] = [1, 2, 3];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 3);
        shim_nsmutabledata_replace_bytes_in_range(obj, 0, 0, 2, 0);
        // bytes should be unchanged (null bytes ptr = no-op for copy)
        let bytes = unsafe { read_bytes(obj, 3) };
        assert_eq!(bytes, vec![1, 2, 3]);
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn replace_bytes_nil_obj() {
        let r = shim_nsmutabledata_replace_bytes_in_range(0, 0, 0, 5, 0);
        assert_eq!(r, 0);
    }

    // ── -[NSMutableData description] ───────────────────────────────

    #[test]
    fn description_basic() {
        let obj = shim_nsmutabledata_data(0, 0);
        let desc = shim_nsmutabledata_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSMutableData: 0 bytes>");
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn description_with_data() {
        let src: [u8; 5] = [1, 2, 3, 4, 5];
        let obj = shim_nsmutabledata_data_with_bytes(0, 0, src.as_ptr() as u64, 5);
        let desc = shim_nsmutabledata_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSMutableData: 5 bytes>");
        unsafe {
            free_md(obj);
        }
    }

    #[test]
    fn description_nil_returns_zero() {
        assert_eq!(shim_nsmutabledata_description(0, 0), 0);
    }

    // ── Length tracking across mutations ───────────────────────────

    #[test]
    fn length_tracks_mutations() {
        let obj = shim_nsmutabledata_data(0, 0);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 0);

        let a: [u8; 3] = [1, 2, 3];
        shim_nsmutabledata_append_bytes(obj, 0, a.as_ptr() as u64, 3);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 3);

        shim_nsmutabledata_set_length(obj, 0, 10);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 10);

        shim_nsmutabledata_set_length(obj, 0, 1);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 1);

        let b: [u8; 2] = [4, 5];
        shim_nsmutabledata_append_bytes(obj, 0, b.as_ptr() as u64, 2);
        assert_eq!(shim_nsmutabledata_length(obj, 0), 3);

        unsafe {
            free_md(obj);
        }
    }
}
