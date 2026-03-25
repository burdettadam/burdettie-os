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
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
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
