//! Printf-style format string expansion with ObjC `%@` support.

use std::ffi::{c_char, CStr};

use crate::cfstring;
use crate::foundation::cfconstant_string_class_addr;

/// Format an ObjC object for `%@`.
///
/// If the object is a CFConstantString, extract the C string directly.
/// Otherwise, look up and call the object's `-description` method through
/// the ObjC runtime — `description` returns a CFString we can read.
pub(crate) fn format_objc_object(obj: u64) -> String {
    let isa = unsafe { *(obj as *const u64) };

    // Fast path: if it's already a CFString, read directly
    if isa == cfconstant_string_class_addr() {
        if let Some(s) = cfstring::extract_cfstring_cstr(obj) {
            return s.to_string();
        }
    }

    // Slow path: call -description via the ObjC runtime
    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if let Some(rt) = guard.as_ref() {
        if let Some(imp) = rt.msg_lookup(isa, "description") {
            let desc_fn: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(imp) };
            let desc_obj = desc_fn(obj, 0);
            if desc_obj != 0 {
                if let Some(s) = cfstring::extract_cfstring_cstr(desc_obj) {
                    return s.to_string();
                }
            }
        }
    }

    format!("<ObjC object {:#x}>", obj)
}

/// Simple format string expansion.
///
/// Handles `%@` by calling the object's `-description` method (for
/// non-CFString objects) or by directly extracting the C string (for
/// CFConstantString instances). Other specifiers (%d, %s, %ld, %f, etc.)
/// are expanded from the raw argument values.
pub(crate) fn expand_format(fmt: &str, args: &[u64]) -> String {
    let mut result = String::with_capacity(fmt.len());
    let mut chars = fmt.chars().peekable();
    let mut arg_idx = 0;

    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.peek() {
                Some('@') => {
                    chars.next();
                    if arg_idx < args.len() && args[arg_idx] != 0 {
                        result.push_str(&format_objc_object(args[arg_idx]));
                    } else {
                        result.push_str("(null)");
                    }
                    arg_idx += 1;
                }
                Some('d') | Some('i') => {
                    chars.next();
                    if arg_idx < args.len() {
                        result.push_str(&(args[arg_idx] as i64).to_string());
                    }
                    arg_idx += 1;
                }
                Some('u') => {
                    chars.next();
                    if arg_idx < args.len() {
                        result.push_str(&args[arg_idx].to_string());
                    }
                    arg_idx += 1;
                }
                Some('l') => {
                    chars.next();
                    match chars.peek() {
                        Some('l') => {
                            chars.next();
                            match chars.peek() {
                                Some('d') | Some('i') => {
                                    chars.next();
                                    if arg_idx < args.len() {
                                        result.push_str(&(args[arg_idx] as i64).to_string());
                                    }
                                    arg_idx += 1;
                                }
                                Some('u') => {
                                    chars.next();
                                    if arg_idx < args.len() {
                                        result.push_str(&args[arg_idx].to_string());
                                    }
                                    arg_idx += 1;
                                }
                                _ => {
                                    result.push_str("%ll");
                                }
                            }
                        }
                        Some('d') | Some('i') => {
                            chars.next();
                            if arg_idx < args.len() {
                                result.push_str(&(args[arg_idx] as i64).to_string());
                            }
                            arg_idx += 1;
                        }
                        Some('u') => {
                            chars.next();
                            if arg_idx < args.len() {
                                result.push_str(&args[arg_idx].to_string());
                            }
                            arg_idx += 1;
                        }
                        Some('f') => {
                            chars.next();
                            if arg_idx < args.len() {
                                let f: f64 = f64::from_bits(args[arg_idx]);
                                result.push_str(&format!("{:.6}", f));
                            }
                            arg_idx += 1;
                        }
                        _ => {
                            result.push('%');
                            result.push('l');
                        }
                    }
                }
                Some('z') => {
                    chars.next();
                    match chars.peek() {
                        Some('u') => {
                            chars.next();
                            if arg_idx < args.len() {
                                result.push_str(&args[arg_idx].to_string());
                            }
                            arg_idx += 1;
                        }
                        Some('d') | Some('i') => {
                            chars.next();
                            if arg_idx < args.len() {
                                result.push_str(&(args[arg_idx] as i64).to_string());
                            }
                            arg_idx += 1;
                        }
                        _ => {
                            result.push('%');
                            result.push('z');
                        }
                    }
                }
                Some('f') => {
                    chars.next();
                    if arg_idx < args.len() {
                        let f: f64 = f64::from_bits(args[arg_idx]);
                        result.push_str(&format!("{:.6}", f));
                    }
                    arg_idx += 1;
                }
                Some('g') | Some('G') => {
                    let ch = chars.next().expect("already peeked");
                    if arg_idx < args.len() {
                        let f: f64 = f64::from_bits(args[arg_idx]);
                        let s = if ch == 'G' {
                            format!("{:E}", f).replace("E", "E")
                        } else {
                            format!("{:e}", f).replace("e", "e")
                        };
                        let fixed = format!("{}", f);
                        if fixed.len() <= s.len() {
                            result.push_str(&fixed);
                        } else {
                            result.push_str(&s);
                        }
                    }
                    arg_idx += 1;
                }
                Some('e') | Some('E') => {
                    let ch = chars.next().expect("already peeked");
                    if arg_idx < args.len() {
                        let f: f64 = f64::from_bits(args[arg_idx]);
                        if ch == 'E' {
                            result.push_str(&format!("{:.6E}", f));
                        } else {
                            result.push_str(&format!("{:.6e}", f));
                        }
                    }
                    arg_idx += 1;
                }
                Some('x') => {
                    chars.next();
                    if arg_idx < args.len() {
                        result.push_str(&format!("{:x}", args[arg_idx]));
                    }
                    arg_idx += 1;
                }
                Some('X') => {
                    chars.next();
                    if arg_idx < args.len() {
                        result.push_str(&format!("{:X}", args[arg_idx]));
                    }
                    arg_idx += 1;
                }
                Some('o') => {
                    chars.next();
                    if arg_idx < args.len() {
                        result.push_str(&format!("{:o}", args[arg_idx]));
                    }
                    arg_idx += 1;
                }
                Some('p') => {
                    chars.next();
                    if arg_idx < args.len() {
                        result.push_str(&format!("0x{:x}", args[arg_idx]));
                    }
                    arg_idx += 1;
                }
                Some('s') => {
                    chars.next();
                    if arg_idx < args.len() && args[arg_idx] != 0 {
                        let s = unsafe { CStr::from_ptr(args[arg_idx] as *const c_char) };
                        if let Ok(s) = s.to_str() {
                            result.push_str(s);
                        }
                    }
                    arg_idx += 1;
                }
                Some('%') => {
                    chars.next();
                    result.push('%');
                }
                _ => {
                    result.push('%');
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_format_plain_string() {
        assert_eq!(expand_format("hello world", &[]), "hello world");
    }

    #[test]
    fn expand_format_int() {
        assert_eq!(expand_format("value=%d", &[42]), "value=42");
    }

    #[test]
    fn expand_format_percent_escape() {
        assert_eq!(expand_format("100%%", &[]), "100%");
    }

    #[test]
    fn expand_format_obj_null() {
        assert_eq!(expand_format("obj=%@", &[0]), "obj=(null)");
    }

    #[test]
    fn expand_format_multiple_ints() {
        assert_eq!(expand_format("%d + %d = %d", &[3, 4, 7]), "3 + 4 = 7");
    }

    #[test]
    fn expand_format_float() {
        let f: f64 = 2.72;
        assert_eq!(expand_format("pi=%f", &[f.to_bits()]), "pi=2.720000");
    }

    #[test]
    fn expand_format_float_zero() {
        let f: f64 = 0.0;
        assert_eq!(expand_format("%f", &[f.to_bits()]), "0.000000");
    }

    #[test]
    fn expand_format_hex_lower() {
        assert_eq!(expand_format("0x%x", &[255]), "0xff");
    }

    #[test]
    fn expand_format_hex_upper() {
        assert_eq!(expand_format("0x%X", &[255]), "0xFF");
    }

    #[test]
    fn expand_format_pointer() {
        assert_eq!(expand_format("ptr=%p", &[0x1234]), "ptr=0x1234");
    }

    #[test]
    fn expand_format_octal() {
        assert_eq!(expand_format("oct=%o", &[8]), "oct=10");
    }

    #[test]
    fn expand_format_lld() {
        let v = (-123456789i64) as u64;
        assert_eq!(expand_format("big=%lld", &[v]), "big=-123456789");
    }

    #[test]
    fn expand_format_llu() {
        assert_eq!(
            expand_format("big=%llu", &[u64::MAX]),
            format!("big={}", u64::MAX)
        );
    }

    #[test]
    fn expand_format_zu() {
        assert_eq!(expand_format("size=%zu", &[1024]), "size=1024");
    }

    #[test]
    fn expand_format_zd() {
        let v = (-42i64) as u64;
        assert_eq!(expand_format("off=%zd", &[v]), "off=-42");
    }

    #[test]
    fn expand_format_scientific() {
        let f: f64 = 12345.0;
        let s = expand_format("%e", &[f.to_bits()]);
        assert!(
            s.contains("e") || s.contains("E"),
            "expected scientific notation, got: {}",
            s
        );
    }

    #[test]
    fn expand_format_lf() {
        let f: f64 = 2.5;
        assert_eq!(expand_format("val=%lf", &[f.to_bits()]), "val=2.500000");
    }

    #[test]
    fn expand_format_mixed_new_specifiers() {
        let f: f64 = 1.5;
        assert_eq!(
            expand_format("%d %x %f", &[10, 0xFF, f.to_bits()]),
            "10 ff 1.500000"
        );
    }
}
