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
const KIND_URL_QUERY_ALLOWED: u64 = 10;
const KIND_URL_PATH_ALLOWED: u64 = 11;
const KIND_URL_HOST_ALLOWED: u64 = 12;
const KIND_URL_FRAGMENT_ALLOWED: u64 = 13;
const KIND_URL_USER_ALLOWED: u64 = 14;
const KIND_URL_PASSWORD_ALLOWED: u64 = 15;

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
        KIND_URL_QUERY_ALLOWED => {
            // RFC 3986 unreserved + query-allowed: A-Z a-z 0-9 - . _ ~ ! $ & ' ( ) * + , ; = : @ / ?
            ch.is_ascii_alphanumeric()
                || "-._~!$&'()*+,;=:@/?".contains(ch)
        }
        KIND_URL_PATH_ALLOWED => {
            // RFC 3986 unreserved + pchar: A-Z a-z 0-9 - . _ ~ ! $ & ' ( ) * + , ; = : @ /
            ch.is_ascii_alphanumeric()
                || "-._~!$&'()*+,;=:@/".contains(ch)
        }
        KIND_URL_HOST_ALLOWED => {
            // RFC 3986 unreserved + host chars: A-Z a-z 0-9 - . _ ~ ! $ & ' ( ) * + , ; = [ ] :
            ch.is_ascii_alphanumeric()
                || "-._~!$&'()*+,;=[]:".contains(ch)
        }
        KIND_URL_FRAGMENT_ALLOWED => {
            // RFC 3986 unreserved + fragment: A-Z a-z 0-9 - . _ ~ ! $ & ' ( ) * + , ; = : @ / ?
            ch.is_ascii_alphanumeric()
                || "-._~!$&'()*+,;=:@/?".contains(ch)
        }
        KIND_URL_USER_ALLOWED => {
            // RFC 3986 unreserved + userinfo: A-Z a-z 0-9 - . _ ~ ! $ & ' ( ) * + , ; =
            ch.is_ascii_alphanumeric()
                || "-._~!$&'()*+,;=".contains(ch)
        }
        KIND_URL_PASSWORD_ALLOWED => {
            // RFC 3986 unreserved + password: A-Z a-z 0-9 - . _ ~ ! $ & ' ( ) * + , ; =
            ch.is_ascii_alphanumeric()
                || "-._~!$&'()*+,;=".contains(ch)
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

/// `+[NSCharacterSet URLQueryAllowedCharacterSet]`
pub extern "C" fn shim_nscharacterset_url_query_allowed(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_URL_QUERY_ALLOWED)
}

/// `+[NSCharacterSet URLPathAllowedCharacterSet]`
pub extern "C" fn shim_nscharacterset_url_path_allowed(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_URL_PATH_ALLOWED)
}

/// `+[NSCharacterSet URLHostAllowedCharacterSet]`
pub extern "C" fn shim_nscharacterset_url_host_allowed(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_URL_HOST_ALLOWED)
}

/// `+[NSCharacterSet URLFragmentAllowedCharacterSet]`
pub extern "C" fn shim_nscharacterset_url_fragment_allowed(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_URL_FRAGMENT_ALLOWED)
}

/// `+[NSCharacterSet URLUserAllowedCharacterSet]`
pub extern "C" fn shim_nscharacterset_url_user_allowed(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_URL_USER_ALLOWED)
}

/// `+[NSCharacterSet URLPasswordAllowedCharacterSet]`
pub extern "C" fn shim_nscharacterset_url_password_allowed(_cls: u64, _cmd: u64) -> u64 {
    alloc_predefined(KIND_URL_PASSWORD_ALLOWED)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── Predefined set factory tests ──────────────────────────────

    #[test]
    fn whitespace_set_basic() {
        let obj = shim_nscharacterset_whitespace(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nscharacterset_class_addr());
        assert_eq!(read_field(obj, KIND_OFF), KIND_WHITESPACE);
        assert!(character_set_contains(obj, ' '));
        assert!(character_set_contains(obj, '\t'));
        assert!(!character_set_contains(obj, '\n'));
        assert!(!character_set_contains(obj, 'a'));
        free_obj(obj);
    }

    #[test]
    fn whitespace_and_newline_set_basic() {
        let obj = shim_nscharacterset_whitespace_and_newline(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_WHITESPACE_NEWLINE);
        assert!(character_set_contains(obj, ' '));
        assert!(character_set_contains(obj, '\t'));
        assert!(character_set_contains(obj, '\n'));
        assert!(character_set_contains(obj, '\r'));
        assert!(character_set_contains(obj, '\u{2028}'));
        assert!(character_set_contains(obj, '\u{2029}'));
        assert!(!character_set_contains(obj, 'x'));
        free_obj(obj);
    }

    #[test]
    fn decimal_digit_set_basic() {
        let obj = shim_nscharacterset_decimal_digit(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_DECIMAL_DIGIT);
        for c in '0'..='9' {
            assert!(character_set_contains(obj, c), "should contain '{c}'");
        }
        assert!(!character_set_contains(obj, 'a'));
        assert!(!character_set_contains(obj, ' '));
        free_obj(obj);
    }

    #[test]
    fn letter_set_basic() {
        let obj = shim_nscharacterset_letter(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_LETTER);
        assert!(character_set_contains(obj, 'A'));
        assert!(character_set_contains(obj, 'z'));
        assert!(!character_set_contains(obj, '5'));
        assert!(!character_set_contains(obj, ' '));
        free_obj(obj);
    }

    #[test]
    fn lowercase_letter_set_basic() {
        let obj = shim_nscharacterset_lowercase_letter(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_LOWERCASE_LETTER);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, 'z'));
        assert!(!character_set_contains(obj, 'A'));
        assert!(!character_set_contains(obj, '1'));
        free_obj(obj);
    }

    #[test]
    fn uppercase_letter_set_basic() {
        let obj = shim_nscharacterset_uppercase_letter(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_UPPERCASE_LETTER);
        assert!(character_set_contains(obj, 'A'));
        assert!(character_set_contains(obj, 'Z'));
        assert!(!character_set_contains(obj, 'a'));
        assert!(!character_set_contains(obj, '0'));
        free_obj(obj);
    }

    #[test]
    fn alphanumeric_set_basic() {
        let obj = shim_nscharacterset_alphanumeric(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_ALPHANUMERIC);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, 'Z'));
        assert!(character_set_contains(obj, '5'));
        assert!(!character_set_contains(obj, '!'));
        assert!(!character_set_contains(obj, ' '));
        free_obj(obj);
    }

    #[test]
    fn punctuation_set_basic() {
        let obj = shim_nscharacterset_punctuation(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_PUNCTUATION);
        assert!(character_set_contains(obj, '!'));
        assert!(character_set_contains(obj, '.'));
        assert!(character_set_contains(obj, ','));
        assert!(character_set_contains(obj, '@'));
        assert!(!character_set_contains(obj, 'a'));
        assert!(!character_set_contains(obj, '0'));
        free_obj(obj);
    }

    #[test]
    fn newline_set_basic() {
        let obj = shim_nscharacterset_newline(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_NEWLINE);
        assert!(character_set_contains(obj, '\n'));
        assert!(character_set_contains(obj, '\r'));
        assert!(character_set_contains(obj, '\u{0085}'));
        assert!(character_set_contains(obj, '\u{2028}'));
        assert!(character_set_contains(obj, '\u{2029}'));
        assert!(!character_set_contains(obj, ' '));
        assert!(!character_set_contains(obj, '\t'));
        free_obj(obj);
    }

    // ── Custom set (characterSetWithCharactersInString:) ──────────

    #[test]
    fn custom_set_basic() {
        let s = make_string("abc");
        let obj = shim_nscharacterset_with_characters_in_string(0, 0, s);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nscharacterset_class_addr());
        assert_eq!(read_field(obj, KIND_OFF), 0); // custom set, kind=0
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, 'b'));
        assert!(character_set_contains(obj, 'c'));
        assert!(!character_set_contains(obj, 'd'));
        assert!(!character_set_contains(obj, 'A'));
        free_obj(s);
        free_obj(obj);
    }

    #[test]
    fn custom_set_nil_string() {
        let obj = shim_nscharacterset_with_characters_in_string(0, 0, 0);
        assert_ne!(obj, 0);
        // Empty custom set — nothing should be a member
        assert!(!character_set_contains(obj, 'a'));
        assert!(!character_set_contains(obj, ' '));
        assert_eq!(read_field(obj, BITMAP_PTR_OFF), 0);
        assert_eq!(read_field(obj, BITMAP_LEN_OFF), 0);
        free_obj(obj);
    }

    #[test]
    fn custom_set_deduplicates() {
        let s = make_string("aabbcc");
        let obj = shim_nscharacterset_with_characters_in_string(0, 0, s);
        assert_ne!(obj, 0);
        // Should have exactly 3 unique codepoints
        assert_eq!(read_field(obj, BITMAP_LEN_OFF), 3);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, 'b'));
        assert!(character_set_contains(obj, 'c'));
        free_obj(s);
        free_obj(obj);
    }

    #[test]
    fn custom_set_single_char() {
        let s = make_string("x");
        let obj = shim_nscharacterset_with_characters_in_string(0, 0, s);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, BITMAP_LEN_OFF), 1);
        assert!(character_set_contains(obj, 'x'));
        assert!(!character_set_contains(obj, 'y'));
        free_obj(s);
        free_obj(obj);
    }

    // ── characterIsMember: ────────────────────────────────────────

    #[test]
    fn character_is_member_predefined() {
        let obj = shim_nscharacterset_decimal_digit(0, 0);
        assert_eq!(shim_nscharacterset_character_is_member(obj, 0, '3' as u64), 1);
        assert_eq!(shim_nscharacterset_character_is_member(obj, 0, 'a' as u64), 0);
        free_obj(obj);
    }

    #[test]
    fn character_is_member_custom() {
        let s = make_string("xyz");
        let obj = shim_nscharacterset_with_characters_in_string(0, 0, s);
        assert_eq!(shim_nscharacterset_character_is_member(obj, 0, 'x' as u64), 1);
        assert_eq!(shim_nscharacterset_character_is_member(obj, 0, 'w' as u64), 0);
        free_obj(s);
        free_obj(obj);
    }

    #[test]
    fn character_is_member_nil_obj() {
        assert_eq!(shim_nscharacterset_character_is_member(0, 0, 'a' as u64), 0);
    }

    #[test]
    fn character_is_member_invalid_codepoint() {
        let obj = shim_nscharacterset_letter(0, 0);
        // 0xD800 is a surrogate — invalid as a char
        assert_eq!(shim_nscharacterset_character_is_member(obj, 0, 0xD800), 0);
        // Very large invalid codepoint
        assert_eq!(shim_nscharacterset_character_is_member(obj, 0, 0x110000), 0);
        free_obj(obj);
    }

    // ── character_set_contains ────────────────────────────────────

    #[test]
    fn contains_nil_returns_false() {
        assert!(!character_set_contains(0, 'a'));
    }

    #[test]
    fn contains_empty_custom_set() {
        // Manually build a custom set with bitmap_ptr=0, bitmap_len=0
        let obj = alloc_obj(32, nsobj::nscharacterset_class_addr());
        // kind=0 (custom), bitmap_ptr=0, bitmap_len=0 (from calloc zeroing)
        assert!(!character_set_contains(obj, 'a'));
        assert!(!character_set_contains(obj, '\n'));
        free_obj(obj);
    }

    #[test]
    fn contains_unknown_kind_returns_false() {
        let obj = alloc_obj(32, nsobj::nscharacterset_class_addr());
        write_field(obj, KIND_OFF, 999); // unknown predefined kind
        assert!(!character_set_contains(obj, 'a'));
        assert!(!character_set_contains(obj, ' '));
        free_obj(obj);
    }

    // ── URL character set factories ───────────────────────────────

    #[test]
    fn url_query_allowed_set() {
        let obj = shim_nscharacterset_url_query_allowed(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_URL_QUERY_ALLOWED);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, '?'));
        assert!(character_set_contains(obj, '/'));
        assert!(character_set_contains(obj, '='));
        assert!(!character_set_contains(obj, ' '));
        assert!(!character_set_contains(obj, '#'));
        free_obj(obj);
    }

    #[test]
    fn url_path_allowed_set() {
        let obj = shim_nscharacterset_url_path_allowed(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_URL_PATH_ALLOWED);
        assert!(character_set_contains(obj, '/'));
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, ':'));
        assert!(!character_set_contains(obj, '?'));
        assert!(!character_set_contains(obj, ' '));
        free_obj(obj);
    }

    #[test]
    fn url_host_allowed_set() {
        let obj = shim_nscharacterset_url_host_allowed(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_URL_HOST_ALLOWED);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, '['));
        assert!(character_set_contains(obj, ']'));
        assert!(!character_set_contains(obj, '/'));
        assert!(!character_set_contains(obj, ' '));
        free_obj(obj);
    }

    #[test]
    fn url_fragment_allowed_set() {
        let obj = shim_nscharacterset_url_fragment_allowed(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_URL_FRAGMENT_ALLOWED);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, '?'));
        assert!(character_set_contains(obj, '/'));
        assert!(!character_set_contains(obj, ' '));
        assert!(!character_set_contains(obj, '#'));
        free_obj(obj);
    }

    #[test]
    fn url_user_allowed_set() {
        let obj = shim_nscharacterset_url_user_allowed(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_URL_USER_ALLOWED);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, '-'));
        assert!(!character_set_contains(obj, '/'));
        assert!(!character_set_contains(obj, '@'));
        free_obj(obj);
    }

    #[test]
    fn url_password_allowed_set() {
        let obj = shim_nscharacterset_url_password_allowed(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, KIND_OFF), KIND_URL_PASSWORD_ALLOWED);
        assert!(character_set_contains(obj, 'a'));
        assert!(character_set_contains(obj, '~'));
        assert!(!character_set_contains(obj, '@'));
        assert!(!character_set_contains(obj, '/'));
        free_obj(obj);
    }
}
