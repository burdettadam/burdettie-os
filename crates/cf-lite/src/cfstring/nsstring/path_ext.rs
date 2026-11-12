//! NSString path utilities, CharacterSet operations, and additional methods.

use std::ffi::CString;
use std::path::{Component, Path, PathBuf};

use super::basic::{shim_nsstring_is_equal_to_string, shim_nsstring_string_with_format_inner};
use super::search::{NSRange, NS_NOT_FOUND};
use crate::cfstring::{
    alloc_nsstring_from_cstr, extract_cfstring_cstr, shim_cf_string_create_with_cstring,
};

fn normalize_path_string(path: &str, resolve_symlinks: bool) -> String {
    if resolve_symlinks {
        if let Ok(canonical) = std::fs::canonicalize(path) {
            return canonical.to_string_lossy().into_owned();
        }

        let source = Path::new(path);
        let mut resolved = if source.is_absolute() {
            PathBuf::from("/")
        } else {
            PathBuf::new()
        };

        for component in source.components() {
            match component {
                Component::RootDir => resolved = PathBuf::from("/"),
                Component::CurDir => {}
                Component::ParentDir => {
                    resolved.pop();
                }
                Component::Normal(part) => {
                    resolved.push(part);
                    if let Ok(meta) = std::fs::symlink_metadata(&resolved) {
                        if meta.file_type().is_symlink() {
                            if let Ok(target) = std::fs::read_link(&resolved) {
                                resolved = if target.is_absolute() {
                                    target
                                } else {
                                    resolved
                                        .parent()
                                        .map(Path::to_path_buf)
                                        .unwrap_or_default()
                                        .join(target)
                                };
                            }
                        }
                    }
                }
                Component::Prefix(prefix) => {
                    resolved = PathBuf::from(prefix.as_os_str());
                }
            }
        }

        let resolved_string = resolved.to_string_lossy().into_owned();
        if !resolved_string.is_empty() {
            return resolved_string;
        }
    }

    let source = Path::new(path);
    let mut parts: Vec<String> = Vec::new();
    let mut saw_root = false;

    for component in source.components() {
        match component {
            Component::RootDir => {
                saw_root = true;
                parts.clear();
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(last) = parts.last() {
                    if last != ".." {
                        parts.pop();
                    } else if !saw_root {
                        parts.push("..".to_string());
                    }
                } else if !saw_root {
                    parts.push("..".to_string());
                }
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::Prefix(prefix) => {
                saw_root = true;
                parts.clear();
                parts.push(prefix.as_os_str().to_string_lossy().into_owned());
            }
        }
    }

    let mut normalized = if saw_root {
        format!("/{}", parts.join("/"))
    } else {
        parts.join("/")
    };

    if normalized.is_empty() {
        normalized = if saw_root {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }

    normalized
}

// ── Path utility methods ───────────────────────────────────────────

/// `-[NSString path]` — path-like strings are already path values.
pub extern "C" fn shim_nsstring_path(obj: u64, _sel: u64) -> u64 {
    obj
}

/// `-[NSString pathExtension]` — returns extension after last '.'.
pub extern "C" fn shim_nsstring_path_extension(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let last = s.rsplit('/').next().unwrap_or(s);
    let ext = match last.rfind('.') {
        Some(dot) if dot > 0 => &last[dot + 1..],
        _ => "",
    };
    let cstr = CString::new(ext).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString lastPathComponent]` — returns component after last '/'.
pub extern "C" fn shim_nsstring_last_path_component(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let trimmed = s.trim_end_matches('/');
    let comp = if trimmed.is_empty() {
        "/"
    } else {
        trimmed.rsplit('/').next().unwrap_or(trimmed)
    };
    let cstr = CString::new(comp).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByDeletingLastPathComponent]`
pub extern "C" fn shim_nsstring_string_by_deleting_last_path_component(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let trimmed = if s.chars().all(|ch| ch == '/') {
        "/"
    } else {
        s.trim_end_matches('/')
    };
    let result = if trimmed == "/" {
        "/"
    } else {
        match trimmed.rfind('/') {
            Some(0) => "/",
            Some(pos) => &trimmed[..pos],
            None => "",
        }
    };
    let cstr = CString::new(result).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByDeletingPathExtension]`
pub extern "C" fn shim_nsstring_string_by_deleting_path_extension(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let last_slash = s.rfind('/').map(|p| p + 1).unwrap_or(0);
    let result = match s[last_slash..].rfind('.') {
        Some(dot) if dot > 0 => &s[..last_slash + dot],
        _ => s,
    };
    let cstr = CString::new(result).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByAppendingPathComponent:]`
pub extern "C" fn shim_nsstring_string_by_appending_path_component(
    obj: u64,
    _sel: u64,
    component: u64,
) -> u64 {
    let Some(base) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let Some(comp) = extract_cfstring_cstr(component) else {
        return 0;
    };
    let trimmed_base = base.trim_end_matches('/');
    let trimmed_comp = comp.trim_start_matches('/');
    let result = format!("{trimmed_base}/{trimmed_comp}");
    let cstr = CString::new(result).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString isAbsolutePath]`
pub extern "C" fn shim_nsstring_is_absolute_path(obj: u64, _sel: u64) -> u8 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    u8::from(s.starts_with('/'))
}

/// `-[NSString stringByResolvingSymlinksInPath]`
pub extern "C" fn shim_nsstring_string_by_resolving_symlinks_in_path(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let resolved = normalize_path_string(s, true);
    let cstr = CString::new(resolved).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByStandardizingPath]`
pub extern "C" fn shim_nsstring_string_by_standardizing_path(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let standardized = normalize_path_string(s, false);
    let cstr = CString::new(standardized).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

// ── CharacterSet operations ────────────────────────────────────────

/// `-[NSString stringByTrimmingCharactersInSet:]`
pub extern "C" fn shim_nsstring_string_by_trimming_characters_in_set(
    obj: u64,
    _sel: u64,
    charset: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    if charset == 0 {
        return obj;
    }
    let trimmed: String = s
        .trim_start_matches(|c: char| crate::nscharacterset::character_set_contains(charset, c))
        .trim_end_matches(|c: char| crate::nscharacterset::character_set_contains(charset, c))
        .to_string();
    let cstr = CString::new(trimmed).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString componentsSeparatedByCharactersInSet:]`
pub extern "C" fn shim_nsstring_components_separated_by_characters_in_set(
    obj: u64,
    _sel: u64,
    charset: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    if charset == 0 {
        return crate::nsarray::shim_nsarray_array_with_objects_count(
            crate::nsobj::nsarray_class_addr(),
            0,
            &obj as *const u64 as u64,
            1,
        );
    }
    let parts: Vec<&str> = s
        .split(|c: char| crate::nscharacterset::character_set_contains(charset, c))
        .collect();
    let ns_parts: Vec<u64> = parts
        .iter()
        .map(|p| {
            let cstr = CString::new(*p).unwrap_or_default();
            alloc_nsstring_from_cstr(cstr.as_ptr())
        })
        .collect();
    let buf = ns_parts.as_ptr() as u64;
    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        buf,
        ns_parts.len() as u64,
    )
}

/// `-[NSString rangeOfCharacterFromSet:]`
pub extern "C" fn shim_nsstring_range_of_character_from_set(
    obj: u64,
    _sel: u64,
    charset: u64,
) -> NSRange {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    };
    if charset == 0 {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    }
    for (i, ch) in s.char_indices() {
        if crate::nscharacterset::character_set_contains(charset, ch) {
            return NSRange {
                location: i as u64,
                length: ch.len_utf8() as u64,
            };
        }
    }
    NSRange {
        location: NS_NOT_FOUND,
        length: 0,
    }
}

// ── Additional methods ─────────────────────────────────────────────

/// `-[NSString containsString:]`
pub extern "C" fn shim_nsstring_contains_string(obj: u64, _sel: u64, search: u64) -> u8 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let Some(needle) = extract_cfstring_cstr(search) else {
        return 0;
    };
    if s.contains(needle) {
        1
    } else {
        0
    }
}

/// `-[NSString substringWithRange:]`
///
/// On aarch64, NSRange is passed in x2+x3 (two 64-bit registers).
pub extern "C" fn shim_nsstring_substring_with_range(
    obj: u64,
    _sel: u64,
    location: u64,
    length: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let loc = location as usize;
    let len = length as usize;
    if loc + len > s.len() {
        log::warn!(
            "substringWithRange: range {},{} out of bounds for len={}",
            loc,
            len,
            s.len()
        );
        return 0;
    }
    let sub = &s[loc..loc + len];
    eprintln!("[substringWithRange] loc={} len={} → {:?}", loc, len, sub);
    let c_str = CString::new(sub).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString characterAtIndex:]`
pub extern "C" fn shim_nsstring_character_at_index(obj: u64, _sel: u64, index: u64) -> u16 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let idx = index as usize;
    if idx < utf16.len() {
        let ch = utf16[idx];
        eprintln!(
            "[charAtIndex] idx={} → '{}' (0x{:x})",
            idx,
            char::from_u32(ch as u32).unwrap_or('?'),
            ch
        );
        ch
    } else {
        log::warn!(
            "characterAtIndex: index {} out of bounds for len={}",
            idx,
            utf16.len()
        );
        0
    }
}

/// `-[NSString initWithFormat:]` — variadic, handled via same trampoline as stringWithFormat:
#[allow(dead_code)]
pub extern "C" fn shim_nsstring_init_with_format(
    _obj: u64,
    _sel: u64,
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    shim_nsstring_string_with_format_inner(format, a1, a2, a3)
}

/// `-[NSString initWithFormat:arguments:]` — takes a `va_list` pointer.
///
/// On ARM64, `va_list` is `char *` (pointer to an array of stack-saved args).
/// We read up to 3 u64 args from the va_list to pass to our format expander.
pub extern "C" fn shim_nsstring_init_with_format_arguments(
    _obj: u64,
    _sel: u64,
    format: u64,
    va_list: u64,
) -> u64 {
    if format == 0 {
        return 0;
    }
    let (a1, a2, a3) = if va_list != 0 {
        let ptr = va_list as *const u64;
        unsafe { (*ptr, *ptr.add(1), *ptr.add(2)) }
    } else {
        (0, 0, 0)
    };
    shim_nsstring_string_with_format_inner(format, a1, a2, a3)
}

/// `-[NSString initWithString:]`
pub extern "C" fn shim_nsstring_init_with_string(_obj: u64, _sel: u64, other: u64) -> u64 {
    if other == 0 {
        return 0;
    }
    let Some(s) = extract_cfstring_cstr(other) else {
        return 0;
    };
    let c_str = CString::new(s).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString capitalizedString]`
pub extern "C" fn shim_nsstring_capitalized_string(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch.is_whitespace() || ch == '-' || ch == '_' {
            result.push(ch);
            capitalize_next = true;
        } else if capitalize_next {
            for c in ch.to_uppercase() {
                result.push(c);
            }
            capitalize_next = false;
        } else {
            for c in ch.to_lowercase() {
                result.push(c);
            }
        }
    }
    let c_str = CString::new(result).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString caseInsensitiveCompare:]`
pub extern "C" fn shim_nsstring_case_insensitive_compare(obj: u64, _sel: u64, other: u64) -> i64 {
    let a = extract_cfstring_cstr(obj).unwrap_or("");
    let b = extract_cfstring_cstr(other).unwrap_or("");
    let la = a.to_lowercase();
    let lb = b.to_lowercase();
    match la.cmp(&lb) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// `-[NSString isEqual:]` — delegates to isEqualToString: for string objects
pub extern "C" fn shim_nsstring_is_equal(obj: u64, _sel: u64, other: u64) -> u8 {
    if obj == other {
        return 1;
    }
    if other == 0 {
        return 0;
    }
    shim_nsstring_is_equal_to_string(obj, _sel, other)
}

/// `-[NSString stringByExpandingTildeInPath]`
pub extern "C" fn shim_nsstring_string_by_expanding_tilde_in_path(obj: u64, _sel: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    if !s.starts_with('~') {
        return obj;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let expanded = format!("{}{}", home, &s[1..]);
    let cstr = CString::new(expanded).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByAppendingPathExtension:]`
pub extern "C" fn shim_nsstring_string_by_appending_path_extension(
    obj: u64,
    _sel: u64,
    ext: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let Some(e) = extract_cfstring_cstr(ext) else {
        return 0;
    };
    let result = format!("{}.{}", s, e);
    let cstr = CString::new(result).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString pathComponents]` — split path into components as NSArray.
pub extern "C" fn shim_nsstring_path_components(obj: u64, _sel: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let mut components: Vec<&str> = Vec::new();
    if s.starts_with('/') {
        components.push("/");
    }
    for part in s.split('/') {
        if !part.is_empty() {
            components.push(part);
        }
    }
    if components.is_empty() {
        components.push("");
    }
    let ns_parts: Vec<u64> = components
        .iter()
        .map(|p| {
            let cstr = CString::new(*p).unwrap_or_default();
            alloc_nsstring_from_cstr(cstr.as_ptr())
        })
        .collect();
    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        ns_parts.as_ptr() as u64,
        ns_parts.len() as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn path_extension_basic() {
        let s = make_string("/foo/bar.txt");
        let ext = shim_nsstring_path_extension(s, 0);
        assert_string_eq(ext, "txt");
        free_obj(s);
        free_obj(ext);
    }

    #[test]
    fn path_returns_self() {
        let s = make_string("/tmp/example.txt");
        assert_eq!(shim_nsstring_path(s, 0), s);
        free_obj(s);
    }

    #[test]
    fn path_extension_none() {
        let s = make_string("/foo/bar");
        let ext = shim_nsstring_path_extension(s, 0);
        assert_string_eq(ext, "");
        free_obj(s);
        free_obj(ext);
    }

    #[test]
    fn last_path_component() {
        let s = make_string("/usr/local/bin/foo");
        let comp = shim_nsstring_last_path_component(s, 0);
        assert_string_eq(comp, "foo");
        free_obj(s);
        free_obj(comp);
    }

    #[test]
    fn string_by_deleting_last_path_component() {
        let s = make_string("/usr/local/bin/foo");
        let parent = shim_nsstring_string_by_deleting_last_path_component(s, 0);
        assert_string_eq(parent, "/usr/local/bin");
        free_obj(s);
        free_obj(parent);
    }

    #[test]
    fn string_by_deleting_last_path_component_stops_at_root() {
        let s = make_string("/");
        let parent = shim_nsstring_string_by_deleting_last_path_component(s, 0);
        assert_string_eq(parent, "/");
        free_obj(s);
        free_obj(parent);
    }

    #[test]
    fn string_by_deleting_path_extension() {
        let s = make_string("/foo/bar.txt");
        let no_ext = shim_nsstring_string_by_deleting_path_extension(s, 0);
        assert_string_eq(no_ext, "/foo/bar");
        free_obj(s);
        free_obj(no_ext);
    }

    #[test]
    fn string_by_appending_path_component() {
        let base = make_string("/usr/local");
        let comp = make_string("bin");
        let result = shim_nsstring_string_by_appending_path_component(base, 0, comp);
        assert_string_eq(result, "/usr/local/bin");
        free_obj(base);
        free_obj(comp);
        free_obj(result);
    }

    #[test]
    fn is_absolute_path() {
        let abs = make_string("/usr/bin");
        let rel = make_string("usr/bin");
        assert_eq!(shim_nsstring_is_absolute_path(abs, 0), 1);
        assert_eq!(shim_nsstring_is_absolute_path(rel, 0), 0);
        assert_eq!(shim_nsstring_is_absolute_path(0, 0), 0);
        free_obj(abs);
        free_obj(rel);
    }

    #[test]
    fn string_by_standardizing_path_collapses_components() {
        let s = make_string("/tmp/foo/../bar/./baz");
        let result = shim_nsstring_string_by_standardizing_path(s, 0);
        assert_string_eq(result, "/tmp/bar/baz");
        free_obj(s);
        free_obj(result);
    }

    #[cfg(unix)]
    #[test]
    fn string_by_resolving_symlinks_in_path_canonicalizes_when_possible() {
        let tempdir =
            std::env::temp_dir().join(format!("cf-lite-nsstring-path-ext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tempdir);
        std::fs::create_dir_all(tempdir.join("real")).expect("create temp tree");
        let link = tempdir.join("link");
        std::os::unix::fs::symlink(tempdir.join("real"), &link).expect("create symlink");

        let input = make_string(&format!("{}/file/..", link.display()));
        let result = shim_nsstring_string_by_resolving_symlinks_in_path(input, 0);
        assert_string_eq(result, &tempdir.join("real").display().to_string());

        free_obj(input);
        free_obj(result);
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsstring_path(0, 0), 0);
        assert_eq!(shim_nsstring_path_extension(0, 0), 0);
        assert_eq!(shim_nsstring_last_path_component(0, 0), 0);
        assert_eq!(
            shim_nsstring_string_by_deleting_last_path_component(0, 0),
            0
        );
        assert_eq!(shim_nsstring_string_by_deleting_path_extension(0, 0), 0);
        assert_eq!(shim_nsstring_string_by_resolving_symlinks_in_path(0, 0), 0);
        assert_eq!(shim_nsstring_string_by_standardizing_path(0, 0), 0);
        assert_eq!(shim_nsstring_string_by_expanding_tilde_in_path(0, 0), 0);
        assert_eq!(shim_nsstring_string_by_appending_path_extension(0, 0, 0), 0);
        assert_eq!(shim_nsstring_path_components(0, 0), 0);
    }

    #[test]
    fn expanding_tilde_replaces_home() {
        let s = make_string("~/Documents");
        let result = shim_nsstring_string_by_expanding_tilde_in_path(s, 0);
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        assert_string_eq(result, &format!("{}/Documents", home));
        free_obj(s);
        free_obj(result);
    }

    #[test]
    fn expanding_tilde_no_tilde_returns_self() {
        let s = make_string("/usr/bin");
        let result = shim_nsstring_string_by_expanding_tilde_in_path(s, 0);
        assert_eq!(result, s);
        free_obj(s);
    }

    #[test]
    fn expanding_tilde_just_tilde() {
        let s = make_string("~");
        let result = shim_nsstring_string_by_expanding_tilde_in_path(s, 0);
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        assert_string_eq(result, &home);
        free_obj(s);
        free_obj(result);
    }

    #[test]
    fn appending_path_extension_basic() {
        let s = make_string("/foo/bar");
        let ext = make_string("txt");
        let result = shim_nsstring_string_by_appending_path_extension(s, 0, ext);
        assert_string_eq(result, "/foo/bar.txt");
        free_obj(s);
        free_obj(ext);
        free_obj(result);
    }

    #[test]
    fn appending_path_extension_nil_ext() {
        let s = make_string("/foo/bar");
        assert_eq!(shim_nsstring_string_by_appending_path_extension(s, 0, 0), 0);
        free_obj(s);
    }

    #[test]
    fn path_components_absolute() {
        let s = make_string("/usr/local/bin");
        let arr = shim_nsstring_path_components(s, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 4);
        assert_string_eq(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0), "/");
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 1),
            "usr",
        );
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 2),
            "local",
        );
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 3),
            "bin",
        );
        free_obj(s);
    }

    #[test]
    fn path_components_relative() {
        let s = make_string("foo/bar");
        let arr = shim_nsstring_path_components(s, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0),
            "foo",
        );
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 1),
            "bar",
        );
        free_obj(s);
    }

    #[test]
    fn path_components_root() {
        let s = make_string("/");
        let arr = shim_nsstring_path_components(s, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        assert_string_eq(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0), "/");
        free_obj(s);
    }
}
