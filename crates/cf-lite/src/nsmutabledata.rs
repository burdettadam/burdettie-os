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
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmutabledata_class_addr();
    }
    let real_cap = if cap == 0 { INITIAL_CAP } else { cap };
    let buf = crate::pal_alloc().calloc(real_cap as usize, 1) as u64;
    if buf != 0 {
        unsafe { *((obj as usize + BYTES_OFF) as *mut u64) = buf; }
    }
    unsafe { *((obj as usize + CAP_OFF) as *mut u64) = real_cap; }
    obj
}

unsafe fn grow_if_needed(obj: u64, needed: u64) {
    let len = *((obj as usize + LENGTH_OFF) as *const u64);
    let cap = *((obj as usize + CAP_OFF) as *const u64);
    let required = len + needed;
    if required <= cap { return; }
    let mut new_cap = if cap == 0 { INITIAL_CAP } else { cap };
    while new_cap < required { new_cap *= 2; }
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
        unsafe { *((obj as usize + LENGTH_OFF) as *mut u64) = length; }
    }
    obj
}

/// `+[NSMutableData dataWithBytes:length:]`
pub extern "C" fn shim_nsmutabledata_data_with_bytes(
    _cls: u64, _cmd: u64, bytes: u64, length: u64,
) -> u64 {
    let obj = alloc_mutable_data(length);
    if obj == 0 || length == 0 || bytes == 0 { return obj; }
    let buf = unsafe { *((obj as usize + BYTES_OFF) as *const u64) };
    if buf != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes as *const u8, buf as *mut u8, length as usize);
        }
    }
    unsafe { *((obj as usize + LENGTH_OFF) as *mut u64) = length; }
    obj
}

// ── Instance methods (inherited from NSData) ───────────────────────

/// `-[NSMutableData length]`
pub extern "C" fn shim_nsmutabledata_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + LENGTH_OFF) as *const u64) }
}

/// `-[NSMutableData bytes]` / `-[NSMutableData mutableBytes]`
pub extern "C" fn shim_nsmutabledata_bytes(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + BYTES_OFF) as *const u64) }
}

// ── Mutation methods ───────────────────────────────────────────────

/// `-[NSMutableData appendBytes:length:]`
pub extern "C" fn shim_nsmutabledata_append_bytes(
    obj: u64, _cmd: u64, bytes: u64, length: u64,
) -> u64 {
    if obj == 0 || length == 0 || bytes == 0 { return 0; }
    unsafe { grow_if_needed(obj, length); }
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
    if obj == 0 || other == 0 { return 0; }
    let other_len = crate::nsdata::shim_nsdata_length(other, 0);
    let other_bytes = crate::nsdata::shim_nsdata_bytes(other, 0);
    if other_len > 0 && other_bytes != 0 {
        shim_nsmutabledata_append_bytes(obj, 0, other_bytes, other_len);
    }
    0
}

/// `-[NSMutableData setLength:]`
pub extern "C" fn shim_nsmutabledata_set_length(obj: u64, _cmd: u64, new_len: u64) -> u64 {
    if obj == 0 { return 0; }
    let old_len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    if new_len > old_len {
        unsafe { grow_if_needed(obj, new_len - old_len); }
    }
    unsafe { *((obj as usize + LENGTH_OFF) as *mut u64) = new_len; }
    0
}

/// `-[NSMutableData resetBytesInRange:]` (loc, len)
pub extern "C" fn shim_nsmutabledata_reset_bytes_in_range(
    obj: u64, _cmd: u64, loc: u64, len: u64,
) -> u64 {
    if obj == 0 || len == 0 { return 0; }
    let buf = unsafe { *((obj as usize + BYTES_OFF) as *const u64) };
    let data_len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    if buf == 0 { return 0; }
    let end = (loc + len).min(data_len);
    if loc < end {
        unsafe {
            std::ptr::write_bytes((buf as usize + loc as usize) as *mut u8, 0, (end - loc) as usize);
        }
    }
    0
}

/// `-[NSMutableData replaceBytesInRange:withBytes:]` (loc, len, bytes)
pub extern "C" fn shim_nsmutabledata_replace_bytes_in_range(
    obj: u64, _cmd: u64, loc: u64, len: u64, bytes: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let buf = unsafe { *((obj as usize + BYTES_OFF) as *const u64) };
    let data_len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    if buf == 0 { return 0; }
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
    if obj == 0 { return 0; }
    let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
    let desc = format!("<NSMutableData: {} bytes>", len);
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
