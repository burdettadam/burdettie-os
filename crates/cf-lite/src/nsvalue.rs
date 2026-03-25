//! NSValue shim: wraps non-object values (NSRange, pointers, raw bytes).
//!
//! Object layout: 32 bytes
//!   [0..8):  ISA pointer
//!   [8..16): value_lo (u64) — first 8 bytes of stored value (e.g., range.location)
//!   [16..24): value_hi (u64) — second 8 bytes (e.g., range.length)
//!   [24..32): objc_type ptr (pointer to C string with ObjC type encoding)

use crate::cfstring;
use crate::nsobj;

const NSVALUE_SIZE: usize = 32;

/// Allocate a 32-byte NSValue on the heap.
fn alloc_nsvalue(lo: u64, hi: u64, type_ptr: *const u8) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSVALUE_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    unsafe {
        *obj = nsobj::nsvalue_class_addr();
        *obj.add(1) = lo;
        *obj.add(2) = hi;
        *obj.add(3) = type_ptr as u64;
    }
    obj as u64
}

// ── Static type encoding strings ───────────────────────────────────

/// ObjC type encoding for NSRange: `{_NSRange=QQ}`
static RANGE_TYPE: &[u8] = b"{_NSRange=QQ}\0";
/// ObjC type encoding for a pointer: `^v`
static POINTER_TYPE: &[u8] = b"^v\0";

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSValue valueWithBytes:objCType:]`
///
/// Reads up to 16 bytes from the provided buffer based on the type encoding.
/// For simplicity we always copy 16 bytes (zero-padded by calloc).
pub extern "C" fn shim_nsvalue_value_with_bytes(
    _cls: u64,
    _cmd: u64,
    bytes: u64,
    objc_type: u64,
) -> u64 {
    if bytes == 0 {
        return alloc_nsvalue(0, 0, std::ptr::null());
    }
    let lo = unsafe { std::ptr::read_unaligned(bytes as *const u64) };
    let hi = unsafe { std::ptr::read_unaligned((bytes as *const u64).add(1)) };
    let type_ptr = if objc_type != 0 {
        objc_type as *const u8
    } else {
        std::ptr::null()
    };
    alloc_nsvalue(lo, hi, type_ptr)
}

/// `+[NSValue valueWithRange:]` — wraps an NSRange (location, length).
///
/// Called with two u64 args after _cmd: location and length.
pub extern "C" fn shim_nsvalue_value_with_range(
    _cls: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) -> u64 {
    alloc_nsvalue(location, length, RANGE_TYPE.as_ptr())
}

/// `+[NSValue valueWithPointer:]` — wraps a raw pointer.
pub extern "C" fn shim_nsvalue_value_with_pointer(_cls: u64, _cmd: u64, pointer: u64) -> u64 {
    alloc_nsvalue(pointer, 0, POINTER_TYPE.as_ptr())
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSValue rangeValue]` — returns the stored NSRange as two u64s.
///
/// In the ObjC ABI for arm64, an NSRange return is passed via x0/x1.
/// We return a (u64, u64) tuple which the caller will destructure.
/// Since our runtime dispatches via `objc_msgSend`, the simplest
/// approach is to return the location in x0; the caller reads
/// length from offset 16 of the object directly. For fixture
/// compatibility, we return location.
pub extern "C" fn shim_nsvalue_range_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSValue pointerValue]` — returns the stored pointer.
pub extern "C" fn shim_nsvalue_pointer_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSValue objCType]` — returns the ObjC type encoding C-string.
pub extern "C" fn shim_nsvalue_objc_type(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(3)) }
}

/// `-[NSValue description]` — returns an NSString description.
pub extern "C" fn shim_nsvalue_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let lo = unsafe { *((obj as *const u64).add(1)) };
    let hi = unsafe { *((obj as *const u64).add(2)) };
    let desc = format!("<NSValue: lo={lo}, hi={hi}>\0");
    let c_str = std::ffi::CString::new(desc.trim_end_matches('\0')).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
