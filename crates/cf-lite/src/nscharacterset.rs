//! NSCharacterSet shim: immutable set of Unicode characters.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa          (u64 — pointer to NSCharacterSet class)
//!   offset  8: kind         (u64 — enumeration of predefined sets, or 0 for custom)
//!   offset 16: bitmap_ptr   (u64 — pointer to sorted u32[] of codepoints, or 0 for predefined)
//!   offset 24: bitmap_len   (u64 — number of codepoints in custom bitmap)
//!
//! Total: 32 bytes per character set object.
//!
//! For predefined sets (whitespace, alphanumeric, etc.), the `kind` field
//! selects the membership test function. For custom sets, the bitmap is
//! a sorted array of codepoints.

use crate::cfstring;
use crate::nsobj;

const KIND_OFF: usize = 8;
const BITMAP_PTR_OFF: usize = 16;
const BITMAP_LEN_OFF: usize = 24;

// Predefined set kinds
const KIND_WHITESPACE: u64 = 1;
const KIND_WHITESPACE_NEWLINE: u64 = 2;
const KIND_DECIMAL_DIGIT: u64 = 3;
const KIND_LETTER: u64 = 4;
const KIND_LOWERCASE_LETTER: u64 = 5;
const KIND_UPPERCASE_LETTER: u64 = 6;
const KIND_ALPHANUMERIC: u64 = 7;
const KIND_PUNCTUATION: u64 = 8;
const KIND_NEWLINE: u64 = 9;

// ── Helpers ────────────────────────────────────────────────────────

fn alloc_predefined(kind: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nscharacterset_class_addr();
        *((obj as usize + KIND_OFF) as *mut u64) = kind;
    }
    obj
}

fn is_member_predefined(kind: u64, ch: char) -> bool {
    match kind {
        KIND_WHITESPACE => ch == ' ' || ch == '\t' || ch == '\u{00A0}' || ch == '\u{2009}',
        KIND_WHITESPACE_NEWLINE => {
            ch == ' '
                || ch == '\t'
                || ch == '\n'
                || ch == '\r'
                || ch == '\u{000B}'
                || ch == '\u{000C}'
                || ch == '\u{0085}'
                || ch == '\u{00A0}'
                || ch == '\u{2028}'
                || ch == '\u{2029}'
        }
        KIND_DECIMAL_DIGIT => ch.is_ascii_digit(),
        KIND_LETTER => ch.is_alphabetic(),
        KIND_LOWERCASE_LETTER => ch.is_lowercase(),
        KIND_UPPERCASE_LETTER => ch.is_uppercase(),
        KIND_ALPHANUMERIC => ch.is_alphanumeric(),
        KIND_PUNCTUATION => ch.is_ascii_punctuation(),
        KIND_NEWLINE => {
            ch == '\n' || ch == '\r' || ch == '\u{0085}' || ch == '\u{2028}' || ch == '\u{2029}'
        }
        _ => false,
    }
}

/// Check if a character is in this character set.
pub fn character_set_contains(obj: u64, ch: char) -> bool {
    if obj == 0 {
        return false;
    }
    let kind = unsafe { *((obj as usize + KIND_OFF) as *const u64) };
    if kind != 0 {
        return is_member_predefined(kind, ch);
    }
    // Custom set: binary search the sorted codepoint array
    let bitmap_ptr = unsafe { *((obj as usize + BITMAP_PTR_OFF) as *const u64) };
    let bitmap_len = unsafe { *((obj as usize + BITMAP_LEN_OFF) as *const u64) } as usize;
    if bitmap_ptr == 0 || bitmap_len == 0 {
        return false;
    }

    let target = ch as u32;
    let slice = unsafe { std::slice::from_raw_parts(bitmap_ptr as *const u32, bitmap_len) };
    slice.binary_search(&target).is_ok()
}

// ── Class methods (predefined sets) ────────────────────────────────

/// `+[NSCharacterSet whitespaceCharacterSet]`
pub extern "C" fn shim_nscharacterset_whitespace(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_WHITESPACE)
}

/// `+[NSCharacterSet whitespaceAndNewlineCharacterSet]`
pub extern "C" fn shim_nscharacterset_whitespace_and_newline(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_WHITESPACE_NEWLINE)
}

/// `+[NSCharacterSet decimalDigitCharacterSet]`
pub extern "C" fn shim_nscharacterset_decimal_digit(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_DECIMAL_DIGIT)
}

/// `+[NSCharacterSet letterCharacterSet]`
pub extern "C" fn shim_nscharacterset_letter(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_LETTER)
}

/// `+[NSCharacterSet lowercaseLetterCharacterSet]`
pub extern "C" fn shim_nscharacterset_lowercase_letter(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_LOWERCASE_LETTER)
}

/// `+[NSCharacterSet uppercaseLetterCharacterSet]`
pub extern "C" fn shim_nscharacterset_uppercase_letter(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_UPPERCASE_LETTER)
}

/// `+[NSCharacterSet alphanumericCharacterSet]`
pub extern "C" fn shim_nscharacterset_alphanumeric(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_ALPHANUMERIC)
}

/// `+[NSCharacterSet punctuationCharacterSet]`
pub extern "C" fn shim_nscharacterset_punctuation(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_PUNCTUATION)
}

/// `+[NSCharacterSet newlineCharacterSet]`
pub extern "C" fn shim_nscharacterset_newline(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_NEWLINE)
}

/// `+[NSCharacterSet characterSetWithCharactersInString:]`
pub extern "C" fn shim_nscharacterset_with_characters_in_string(
    _cls: u64,
    _cmd: u64,
    string: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nscharacterset_class_addr();
    }

    if string == 0 {
        return obj; // empty custom set
    }

    if let Some(s) = cfstring::extract_cfstring_cstr(string) {
        // Collect unique codepoints, sort them for binary search
        let mut codepoints: Vec<u32> = s.chars().map(|c| c as u32).collect();
        codepoints.sort_unstable();
        codepoints.dedup();

        let len = codepoints.len();
        if len > 0 {
            let buf = crate::pal_alloc().malloc(len * 4) as *mut u32;
            if !buf.is_null() {
                for (i, &cp) in codepoints.iter().enumerate() {
                    unsafe {
                        *buf.add(i) = cp;
                    }
                }
                unsafe {
                    *((obj as usize + BITMAP_PTR_OFF) as *mut u64) = buf as u64;
                    *((obj as usize + BITMAP_LEN_OFF) as *mut u64) = len as u64;
                }
            }
        }
    }

    obj
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSCharacterSet characterIsMember:]` — takes a unichar (u16).
pub extern "C" fn shim_nscharacterset_character_is_member(obj: u64, _cmd: u64, ch: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    if let Some(c) = char::from_u32(ch as u32) {
        character_set_contains(obj, c) as u64
    } else {
        0
    }
}
