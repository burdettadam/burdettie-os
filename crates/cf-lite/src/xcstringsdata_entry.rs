//! XCStringsdataEntry shim: represents a single entry from XCStringsParser.framework.
//!
//! Object layout: 56 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  key (NSString*)
//!   [16..24): value (NSString*)
//!   [24..32): comment (NSString*)
//!   [32..40): commentary (NSString*)
//!   [40..48): commentary_description (NSString*)
//!   [48..56): attributed (u64, treated as BOOL)

const KEY_OFF: usize = 8;
const VALUE_OFF: usize = 16;
const COMMENT_OFF: usize = 24;
const COMMENTARY_OFF: usize = 32;
const COMMENTARY_DESC_OFF: usize = 40;
const ATTRIBUTED_OFF: usize = 48;

/// `-[XCStringsdataEntry initWithKey:value:comment:]`
///
/// Stores key, value, comment at their offsets; attributed defaults to 0.
/// Returns `obj`.
pub extern "C" fn shim_xcstringsdataentry_init_with_key_value_comment(
    obj: u64,
    _cmd: u64,
    key: u64,
    value: u64,
    comment: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + KEY_OFF) as *mut u64) = key;
        *((obj as usize + VALUE_OFF) as *mut u64) = value;
        *((obj as usize + COMMENT_OFF) as *mut u64) = comment;
        *((obj as usize + ATTRIBUTED_OFF) as *mut u64) = 0;
    }
    obj
}

/// `-[XCStringsdataEntry key]`
pub extern "C" fn shim_xcstringsdataentry_key(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + KEY_OFF) as *const u64) }
}

/// `-[XCStringsdataEntry value]`
pub extern "C" fn shim_xcstringsdataentry_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + VALUE_OFF) as *const u64) }
}

/// `-[XCStringsdataEntry comment]`
pub extern "C" fn shim_xcstringsdataentry_comment(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + COMMENT_OFF) as *const u64) }
}

/// `-[XCStringsdataEntry attributed]`
pub extern "C" fn shim_xcstringsdataentry_attributed(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + ATTRIBUTED_OFF) as *const u64) }
}

/// `-[XCStringsdataEntry setAttributed:]`
pub extern "C" fn shim_xcstringsdataentry_set_attributed(obj: u64, _cmd: u64, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + ATTRIBUTED_OFF) as *mut u64) = val;
    }
}

/// `-[XCStringsdataEntry setKey:]`
pub extern "C" fn shim_xcstringsdataentry_set_key(obj: u64, _cmd: u64, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + KEY_OFF) as *mut u64) = val;
    }
}

/// `-[XCStringsdataEntry setValue:]`
pub extern "C" fn shim_xcstringsdataentry_set_value(obj: u64, _cmd: u64, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + VALUE_OFF) as *mut u64) = val;
    }
}

/// `-[XCStringsdataEntry setCommentary:]`
pub extern "C" fn shim_xcstringsdataentry_set_commentary(obj: u64, _cmd: u64, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + COMMENTARY_OFF) as *mut u64) = val;
    }
}

/// `-[XCStringsdataEntry commentary]`
pub extern "C" fn shim_xcstringsdataentry_commentary(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + COMMENTARY_OFF) as *const u64) }
}

/// `-[XCStringsdataEntry commentaryDescription]`
pub extern "C" fn shim_xcstringsdataentry_commentary_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + COMMENTARY_DESC_OFF) as *const u64) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const SIZE: usize = 56;
    const ISA: u64 = 0xDEAD_E001;

    #[test]
    fn init_stores_fields() {
        let obj = alloc_obj(SIZE, ISA);
        let k = make_string("greeting");
        let v = make_string("Hello");
        let c = make_string("A greeting message");

        let ret = shim_xcstringsdataentry_init_with_key_value_comment(obj, 0, k, v, c);
        assert_eq!(ret, obj);

        assert_string_eq(shim_xcstringsdataentry_key(obj, 0), "greeting");
        assert_string_eq(shim_xcstringsdataentry_value(obj, 0), "Hello");
        assert_string_eq(
            shim_xcstringsdataentry_comment(obj, 0),
            "A greeting message",
        );

        free_obj(obj);
    }

    #[test]
    fn setters_update_fields() {
        let obj = alloc_obj(SIZE, ISA);
        shim_xcstringsdataentry_init_with_key_value_comment(obj, 0, 0, 0, 0);

        let k = make_string("new_key");
        shim_xcstringsdataentry_set_key(obj, 0, k);
        assert_string_eq(shim_xcstringsdataentry_key(obj, 0), "new_key");

        let v = make_string("new_value");
        shim_xcstringsdataentry_set_value(obj, 0, v);
        assert_string_eq(shim_xcstringsdataentry_value(obj, 0), "new_value");

        shim_xcstringsdataentry_set_attributed(obj, 0, 1);
        assert_eq!(shim_xcstringsdataentry_attributed(obj, 0), 1);

        let cm = make_string("some commentary");
        shim_xcstringsdataentry_set_commentary(obj, 0, cm);
        assert_string_eq(
            shim_xcstringsdataentry_commentary(obj, 0),
            "some commentary",
        );

        free_obj(obj);
    }

    #[test]
    fn nil_safety() {
        // Getters on nil return 0
        assert_eq!(shim_xcstringsdataentry_key(0, 0), 0);
        assert_eq!(shim_xcstringsdataentry_value(0, 0), 0);
        assert_eq!(shim_xcstringsdataentry_comment(0, 0), 0);
        assert_eq!(shim_xcstringsdataentry_attributed(0, 0), 0);
        assert_eq!(shim_xcstringsdataentry_commentary(0, 0), 0);
        assert_eq!(shim_xcstringsdataentry_commentary_description(0, 0), 0);

        // Setters on nil don't crash
        shim_xcstringsdataentry_set_key(0, 0, 42);
        shim_xcstringsdataentry_set_value(0, 0, 42);
        shim_xcstringsdataentry_set_attributed(0, 0, 1);
        shim_xcstringsdataentry_set_commentary(0, 0, 42);

        // Init on nil returns 0
        assert_eq!(
            shim_xcstringsdataentry_init_with_key_value_comment(0, 0, 1, 2, 3),
            0
        );
    }

    #[test]
    fn attributed_defaults_to_zero() {
        let obj = alloc_obj(SIZE, ISA);
        let k = make_string("k");
        let v = make_string("v");
        let c = make_string("c");
        shim_xcstringsdataentry_init_with_key_value_comment(obj, 0, k, v, c);

        assert_eq!(shim_xcstringsdataentry_attributed(obj, 0), 0);

        free_obj(obj);
    }

    #[test]
    fn commentary_description_initially_nil() {
        let obj = alloc_obj(SIZE, ISA);
        shim_xcstringsdataentry_init_with_key_value_comment(obj, 0, 0, 0, 0);

        // commentary_description is not set by init, should be 0 (from calloc zeroing)
        assert_eq!(shim_xcstringsdataentry_commentary_description(obj, 0), 0);

        free_obj(obj);
    }
}
