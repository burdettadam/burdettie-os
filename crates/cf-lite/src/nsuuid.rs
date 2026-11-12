//! NSUUID shim: UUID generation and string conversion.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  uuid_lo (u64) — lower 8 bytes of the 128-bit UUID
//!   [16..24): uuid_hi (u64) — upper 8 bytes of the 128-bit UUID

use crate::cfstring;
use crate::nsobj;

const NSUUID_SIZE: usize = 24;

/// Generate a random 128-bit v4 UUID.
fn generate_uuid_v4() -> (u64, u64) {
    let mut bytes = [0u8; 16];
    crate::platform().rng().fill_bytes(&mut bytes);
    // Set version (4) and variant (RFC 4122)
    bytes[6] = (bytes[6] & 0x0F) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3F) | 0x80; // variant 10xx
    let lo = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    let hi = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    (lo, hi)
}

fn alloc_nsuuid(lo: u64, hi: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSUUID_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    unsafe {
        *obj = nsobj::nsuuid_class_addr();
        *obj.add(1) = lo;
        *obj.add(2) = hi;
    }
    obj as u64
}

/// Format UUID bytes as the canonical 8-4-4-4-12 uppercase hex string.
fn uuid_to_string(lo: u64, hi: u64) -> String {
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&lo.to_le_bytes());
    bytes[8..16].copy_from_slice(&hi.to_le_bytes());
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

/// Parse a UUID string (8-4-4-4-12 hex) into (lo, hi).
fn parse_uuid_string(s: &str) -> Option<(u64, u64)> {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for i in 0..16 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    let lo = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    let hi = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    Some((lo, hi))
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSUUID UUID]` — creates a new random UUID.
pub extern "C" fn shim_nsuuid_uuid(_cls: u64, _cmd: u64) -> u64 {
    let (lo, hi) = generate_uuid_v4();
    alloc_nsuuid(lo, hi)
}

/// `-[NSUUID init]` — initialise an already-alloc'd NSUUID with a random value.
pub extern "C" fn shim_nsuuid_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let (lo, hi) = generate_uuid_v4();
    unsafe {
        let ptr = obj as *mut u64;
        *ptr = nsobj::nsuuid_class_addr();
        *ptr.add(1) = lo;
        *ptr.add(2) = hi;
    }
    obj
}

/// `-[NSUUID initWithUUIDString:]` — init from a UUID string.
pub extern "C" fn shim_nsuuid_init_with_uuid_string(obj: u64, _cmd: u64, string: u64) -> u64 {
    if obj == 0 || string == 0 {
        return 0;
    }
    let Some(cstr) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    if let Some((lo, hi)) = parse_uuid_string(cstr) {
        unsafe {
            let ptr = obj as *mut u64;
            *ptr.add(1) = lo;
            *ptr.add(2) = hi;
        }
        obj
    } else {
        0 // invalid format → return nil
    }
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSUUID UUIDString]` — returns the canonical string representation.
pub extern "C" fn shim_nsuuid_uuid_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let lo = unsafe { *((obj as *const u64).add(1)) };
    let hi = unsafe { *((obj as *const u64).add(2)) };
    let s = uuid_to_string(lo, hi);
    let c_str = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSUUID isEqual:]` — compares two UUIDs.
pub extern "C" fn shim_nsuuid_is_equal(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == 0 || other == 0 {
        return 0;
    }
    unsafe {
        let self_lo = *((obj as *const u64).add(1));
        let self_hi = *((obj as *const u64).add(2));
        let other_lo = *((other as *const u64).add(1));
        let other_hi = *((other as *const u64).add(2));
        u8::from(self_lo == other_lo && self_hi == other_hi)
    }
}

/// `-[NSUUID description]` — returns the UUID string.
pub extern "C" fn shim_nsuuid_description(obj: u64, _cmd: u64) -> u64 {
    shim_nsuuid_uuid_string(obj, _cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // Note: shim_nsuuid_uuid() and shim_nsuuid_init() call crate::platform().rng()
    // and cannot be unit-tested without a platform provider configured.

    fn make_zeroed_uuid() -> u64 {
        alloc_obj(NSUUID_SIZE, crate::nsobj::nsuuid_class_addr())
    }

    #[test]
    fn init_with_uuid_string_roundtrip() {
        let uuid_str = make_string("CAFEBABE-CAFE-4ABE-8ABE-CAFEBABECAFE");
        let obj = make_zeroed_uuid();
        let result = shim_nsuuid_init_with_uuid_string(obj, 0, uuid_str);
        assert_eq!(result, obj, "init should return self on success");
        let out = shim_nsuuid_uuid_string(obj, 0);
        assert_ne!(out, 0);
        assert_string_eq(out, "CAFEBABE-CAFE-4ABE-8ABE-CAFEBABECAFE");
        free_obj(obj);
    }

    #[test]
    fn init_with_invalid_uuid_string_returns_nil() {
        let bad = make_string("not-a-uuid");
        let obj = make_zeroed_uuid();
        let result = shim_nsuuid_init_with_uuid_string(obj, 0, bad);
        assert_eq!(result, 0);
        free_obj(obj);
    }

    #[test]
    fn init_with_nil_string_returns_nil() {
        let obj = make_zeroed_uuid();
        assert_eq!(shim_nsuuid_init_with_uuid_string(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn uuid_string_nil_returns_zero() {
        assert_eq!(shim_nsuuid_uuid_string(0, 0), 0);
    }

    #[test]
    fn is_equal_same_bytes() {
        let s = make_string("AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA");
        let a = make_zeroed_uuid();
        let b = make_zeroed_uuid();
        shim_nsuuid_init_with_uuid_string(a, 0, s);
        shim_nsuuid_init_with_uuid_string(b, 0, s);
        assert_eq!(shim_nsuuid_is_equal(a, 0, b), 1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_different_bytes() {
        let s1 = make_string("AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA");
        let s2 = make_string("BBBBBBBB-BBBB-4BBB-8BBB-BBBBBBBBBBBB");
        let a = make_zeroed_uuid();
        let b = make_zeroed_uuid();
        shim_nsuuid_init_with_uuid_string(a, 0, s1);
        shim_nsuuid_init_with_uuid_string(b, 0, s2);
        assert_eq!(shim_nsuuid_is_equal(a, 0, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_nil_returns_zero() {
        let obj = make_zeroed_uuid();
        assert_eq!(shim_nsuuid_is_equal(obj, 0, 0), 0);
        assert_eq!(shim_nsuuid_is_equal(0, 0, obj), 0);
        free_obj(obj);
    }

    #[test]
    fn description_matches_uuid_string() {
        let s = make_string("12345678-1234-4234-A234-123456789ABC");
        let obj = make_zeroed_uuid();
        shim_nsuuid_init_with_uuid_string(obj, 0, s);
        let desc = shim_nsuuid_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "12345678-1234-4234-A234-123456789ABC");
        free_obj(obj);
    }
}
