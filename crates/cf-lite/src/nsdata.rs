//! NSData shim: immutable byte buffer.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa        (u64 — pointer to NSData class)
//!   offset  8: length     (u64 — byte count)
//!   offset 16: bytes_ptr  (u64 — pointer to malloc'd byte buffer, or 0 for empty)
//!
//! Total: 24 bytes per NSData object.

const NSDATA_LENGTH_OFF: usize = 8;
const NSDATA_BYTES_OFF: usize = 16;

/// Create an NSData from a byte slice.
pub fn make_nsdata(bytes: &[u8]) -> u64 {
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdata_class_addr();
        *((obj as usize + NSDATA_LENGTH_OFF) as *mut u64) = bytes.len() as u64;
    }
    if !bytes.is_empty() {
        let buf = unsafe { libc::malloc(bytes.len()) };
        if !buf.is_null() {
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len()); }
        }
        unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *mut u64) = buf as u64; }
    }
    obj
}

/// `+[NSData dataWithBytes:length:]`
pub extern "C" fn shim_nsdata_data_with_bytes_length(
    _cls: u64,
    _cmd: u64,
    bytes: u64,
    length: u64,
) -> u64 {
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdata_class_addr();
    }
    unsafe {
        *((obj as usize + NSDATA_LENGTH_OFF) as *mut u64) = length;
    }

    if length > 0 && bytes != 0 {
        let buf = unsafe { libc::malloc(length as usize) } as u64;
        if buf != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(bytes as *const u8, buf as *mut u8, length as usize);
            }
        }
        unsafe {
            *((obj as usize + NSDATA_BYTES_OFF) as *mut u64) = buf;
        }
    }

    log::debug!(
        "NSData dataWithBytes:length: → {:#x} ({} bytes)",
        obj,
        length
    );
    obj
}

/// `+[NSData data]` — empty data.
pub extern "C" fn shim_nsdata_data(_cls: u64, _cmd: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdata_class_addr();
    }
    obj
}

/// `-[NSData length]`
pub extern "C" fn shim_nsdata_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) }
}

/// `-[NSData bytes]` — returns raw pointer to byte buffer.
pub extern "C" fn shim_nsdata_bytes(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) }
}

/// `-[NSData description]` — returns NSString like `<hexdump>`.
pub extern "C" fn shim_nsdata_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let length = unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) };

    let hex = if length == 0 || bytes_ptr == 0 {
        String::new()
    } else {
        let slice = unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, length) };
        slice.iter().map(|b| format!("{b:02x}")).collect()
    };

    let desc = format!("<{hex}>");
    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSData isEqualToData:]`
pub extern "C" fn shim_nsdata_is_equal_to_data(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == other {
        return 1;
    }
    if obj == 0 || other == 0 {
        return 0;
    }

    let len_a = unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) };
    let len_b = unsafe { *((other as usize + NSDATA_LENGTH_OFF) as *const u64) };
    if len_a != len_b {
        return 0;
    }
    if len_a == 0 {
        return 1;
    }

    let ptr_a = unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) };
    let ptr_b = unsafe { *((other as usize + NSDATA_BYTES_OFF) as *const u64) };
    if ptr_a == 0 || ptr_b == 0 {
        return 0;
    }

    let slice_a = unsafe { std::slice::from_raw_parts(ptr_a as *const u8, len_a as usize) };
    let slice_b = unsafe { std::slice::from_raw_parts(ptr_b as *const u8, len_a as usize) };
    u8::from(slice_a == slice_b)
}
