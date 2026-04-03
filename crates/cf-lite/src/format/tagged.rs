use std::ffi::{c_char, CStr};

use super::format_objc_object;

const TAG_SHIFT: u64 = 60;
const TAG_MASK: u64 = 0x7;

pub(super) fn format_tagged_pointer(obj: u64) -> String {
    match (obj >> TAG_SHIFT) & TAG_MASK {
        2 => crate::nsnumber::tagged_int_value(obj).to_string(),
        3 => format!("<tagged NSString {:#x}>", obj),
        6 => format!("<tagged NSDate {:#x}>", obj),
        _ => format!("<tagged {:#x}>", obj),
    }
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
    use super::format_tagged_pointer;

    #[test]
    fn tagged_nsnumber_formats_without_host_objc() {
        let tagged_number = (1u64 << 63) | (2u64 << 60) | 123;
        assert_eq!(format_tagged_pointer(tagged_number), "123");
    }

    #[test]
    fn tagged_nsstring_formats_as_guest_placeholder() {
        let tagged_string = (1u64 << 63) | (3u64 << 60) | 0x55;
        assert_eq!(
            format_tagged_pointer(tagged_string),
            format!("<tagged NSString {:#x}>", tagged_string)
        );
    }

    #[test]
    fn unknown_tagged_pointer_falls_back_to_generic_placeholder() {
        let tagged_unknown = (1u64 << 63) | (5u64 << 60) | 0x77;
        assert_eq!(
            format_tagged_pointer(tagged_unknown),
            format!("<tagged {:#x}>", tagged_unknown)
        );
    }
}

