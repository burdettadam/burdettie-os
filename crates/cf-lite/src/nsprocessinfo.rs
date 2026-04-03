//! NSProcessInfo shim: singleton process information provider.
//!
//! Object layout (heap-allocated, singleton):
//!
//!   offset  0: isa        (u64 — pointer to NSProcessInfo class)
//!
//! Total: 8 bytes (all data is pulled from the host process).
//! The singleton is lazily allocated on first `+processInfo` call.

use std::sync::OnceLock;

static PROCESS_INFO_INSTANCE: OnceLock<u64> = OnceLock::new();

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NSOperatingSystemVersion {
    pub major_version: i64,
    pub minor_version: i64,
    pub patch_version: i64,
}

fn current_os_version() -> NSOperatingSystemVersion {
    NSOperatingSystemVersion {
        major_version: 14,
        minor_version: 0,
        patch_version: 0,
    }
}

fn get_or_create_instance() -> u64 {
    *PROCESS_INFO_INSTANCE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = crate::nsobj::nsprocessinfo_class_addr();
            }
        }
        obj
    })
}

/// `+[NSProcessInfo processInfo]` — returns the singleton.
pub extern "C" fn shim_nsprocessinfo_process_info(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_instance()
}

/// `-[NSProcessInfo processName]` — returns process name as NSString.
pub extern "C" fn shim_nsprocessinfo_process_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    // Use the shim config's process name if available, else fall back to "unknown"
    let name = libsystem_shims::process::process_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let cstr = std::ffi::CString::new(name).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSProcessInfo processIdentifier]` — returns PID as int.
pub extern "C" fn shim_nsprocessinfo_process_identifier(obj: u64, _cmd: u64) -> i32 {
    if obj == 0 {
        return 0;
    }
    crate::platform().process().pid()
}

/// `-[NSProcessInfo operatingSystemVersionString]`
pub extern "C" fn shim_nsprocessinfo_os_version_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let cstr = c"Version 14.0 (Build 23A344)";
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSProcessInfo operatingSystemVersion]`
pub extern "C" fn shim_nsprocessinfo_operating_system_version(
    obj: u64,
    _cmd: u64,
) -> NSOperatingSystemVersion {
    if obj == 0 {
        return NSOperatingSystemVersion::default();
    }

    current_os_version()
}

/// `-[NSProcessInfo isOperatingSystemAtLeastVersion:]`
pub extern "C" fn shim_nsprocessinfo_is_operating_system_at_least_version(
    obj: u64,
    _cmd: u64,
    version: NSOperatingSystemVersion,
) -> u8 {
    if obj == 0 {
        return 0;
    }

    let current = current_os_version();
    u8::from(
        (current.major_version, current.minor_version, current.patch_version)
            >= (version.major_version, version.minor_version, version.patch_version),
    )
}

/// `-[NSProcessInfo arguments]` — returns argv as NSArray of NSStrings.
pub extern "C" fn shim_nsprocessinfo_arguments(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let args = libsystem_shims::process::process_args().unwrap_or_else(|| {
        vec![
            libsystem_shims::process::process_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "burdettie-os".to_string()),
        ]
    });

    let ns_args: Vec<u64> = args
        .iter()
        .map(|arg| {
            let cstr = std::ffi::CString::new(arg.as_str()).unwrap_or_default();
            crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
        })
        .collect();

    crate::nsarray::make_nsarray(&ns_args)
}

/// `-[NSProcessInfo environment]` — returns NSDictionary of env vars from the host process.
///
/// Reads `environ` from the host C runtime so that DEVELOPER_DIR, SDKROOT, PATH and any
/// other shell variables are visible to guest code that calls this selector.
pub extern "C" fn shim_nsprocessinfo_environment(_obj: u64, _cmd: u64) -> u64 {
    // Safety: `environ` is a null-terminated array of "KEY=VALUE\0" C strings.
    // We read it once, building NSString pairs before returning.
    let mut pairs: Vec<(String, String)> = Vec::new();
    unsafe {
        extern "C" {
            static environ: *const *const libc::c_char;
        }
        let mut ptr = environ;
        while !ptr.is_null() {
            let entry = *ptr;
            if entry.is_null() { break; }
            let s = std::ffi::CStr::from_ptr(entry).to_string_lossy();
            if let Some(eq) = s.find('=') {
                pairs.push((s[..eq].to_string(), s[eq + 1..].to_string()));
            }
            ptr = ptr.add(1);
        }
    }

    // Build an NSMutableDictionary using the existing public shims.
    let cls = crate::nsobj::nsmutabledictionary_class_addr();
    let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary_with_capacity(
        cls, 0, pairs.len() as u64,
    );
    if dict == 0 { return 0; }

    for (k, v) in &pairs {
        let ck = std::ffi::CString::new(k.as_str()).unwrap_or_default();
        let cv = std::ffi::CString::new(v.as_str()).unwrap_or_default();
        let ns_k = crate::cfstring::shim_cf_string_create_with_cstring(0, ck.as_ptr(), 0);
        let ns_v = crate::cfstring::shim_cf_string_create_with_cstring(0, cv.as_ptr(), 0);
        if ns_k != 0 && ns_v != 0 {
            crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, ns_v, ns_k);
        }
    }

    dict
}

/// `-[NSProcessInfo hostName]`
pub extern "C" fn shim_nsprocessinfo_host_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let cstr = c"localhost";
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSProcessInfo description]`
pub extern "C" fn shim_nsprocessinfo_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let name = libsystem_shims::process::process_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let pid = crate::platform().process().pid();
    let desc = format!("<NSProcessInfo: processName={name}, pid={pid}>");
    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSProcessInfo globallyUniqueString]`
pub extern "C" fn shim_nsprocessinfo_globally_unique_string(_obj: u64, _cmd: u64) -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let pid = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let s = format!("{pid}_{ts}_{seq}");
    let cstr = std::ffi::CString::new(s).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSProcessInfo activeProcessorCount]`
pub extern "C" fn shim_nsprocessinfo_active_processor_count(_obj: u64, _cmd: u64) -> u64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u64)
        .unwrap_or(1)
}

/// `-[NSProcessInfo physicalMemory]`
pub extern "C" fn shim_nsprocessinfo_physical_memory(_obj: u64, _cmd: u64) -> u64 {
    #[cfg(target_os = "macos")]
    {
        unsafe {
            let pages = libc::sysconf(libc::_SC_PHYS_PAGES);
            let page_size = libc::sysconf(libc::_SC_PAGESIZE);
            if pages > 0 && page_size > 0 {
                return (pages as u64).saturating_mul(page_size as u64);
            }
        }
    }
    // Fallback: 16 GB
    17_179_869_184
}

/// `-[NSProcessInfo systemUptime]`
pub extern "C" fn shim_nsprocessinfo_system_uptime(_obj: u64, _cmd: u64) -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // Note: shim_nsprocessinfo_process_identifier() and
    // shim_nsprocessinfo_description() call crate::platform().process().pid()
    // and cannot be tested without a platform provider configured.

    #[test]
    fn process_info_returns_nonzero() {
        assert_ne!(shim_nsprocessinfo_process_info(0, 0), 0);
    }

    #[test]
    fn process_info_is_stable_singleton() {
        let a = shim_nsprocessinfo_process_info(0, 0);
        let b = shim_nsprocessinfo_process_info(0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn os_version_string_is_nonempty() {
        let obj = shim_nsprocessinfo_process_info(0, 0);
        assert_ne!(shim_nsprocessinfo_os_version_string(obj, 0), 0);
    }

    #[test]
    fn operating_system_version_matches_stubbed_version() {
        let obj = shim_nsprocessinfo_process_info(0, 0);
        let version = shim_nsprocessinfo_operating_system_version(obj, 0);
        assert_eq!(
            version,
            NSOperatingSystemVersion {
                major_version: 14,
                minor_version: 0,
                patch_version: 0,
            }
        );
    }

    #[test]
    fn is_operating_system_at_least_version_compares_lexicographically() {
        let obj = shim_nsprocessinfo_process_info(0, 0);
        assert_eq!(
            shim_nsprocessinfo_is_operating_system_at_least_version(
                obj,
                0,
                NSOperatingSystemVersion {
                    major_version: 13,
                    minor_version: 9,
                    patch_version: 9,
                },
            ),
            1
        );
        assert_eq!(
            shim_nsprocessinfo_is_operating_system_at_least_version(
                obj,
                0,
                NSOperatingSystemVersion {
                    major_version: 14,
                    minor_version: 0,
                    patch_version: 1,
                },
            ),
            0
        );
    }

    #[test]
    fn host_name_is_localhost() {
        let obj = shim_nsprocessinfo_process_info(0, 0);
        let hostname = shim_nsprocessinfo_host_name(obj, 0);
        assert_ne!(hostname, 0);
        assert_string_eq(hostname, "localhost");
    }

    #[test]
    fn process_name_returns_nonempty_string() {
        let obj = shim_nsprocessinfo_process_info(0, 0);
        let name = shim_nsprocessinfo_process_name(obj, 0);
        assert_ne!(name, 0);
    }

    #[test]
    fn arguments_has_at_least_one_element() {
        let obj = shim_nsprocessinfo_process_info(0, 0);
        let args = shim_nsprocessinfo_arguments(obj, 0);
        assert_ne!(args, 0);
        // NSArray layout: count at offset 8
        let count = read_field(args, 8);
        assert!(count >= 1, "arguments should include at least argv[0], got {count}");
    }

    #[test]
    fn arguments_first_element_matches_process_name() {
        let obj = shim_nsprocessinfo_process_info(0, 0);
        let args = shim_nsprocessinfo_arguments(obj, 0);
        assert_ne!(args, 0);

        let first = crate::nsarray::shim_nsarray_object_at_index(args, 0, 0);
        assert_ne!(first, 0);
        let expected = libsystem_shims::process::process_name().unwrap_or("burdettie-os");
        assert_string_eq(first, expected);
    }

    #[test]
    fn nil_obj_returns_zero() {
        assert_eq!(shim_nsprocessinfo_os_version_string(0, 0), 0);
        assert_eq!(
            shim_nsprocessinfo_operating_system_version(0, 0),
            NSOperatingSystemVersion::default()
        );
        assert_eq!(
            shim_nsprocessinfo_is_operating_system_at_least_version(
                0,
                0,
                NSOperatingSystemVersion {
                    major_version: 14,
                    minor_version: 0,
                    patch_version: 0,
                },
            ),
            0
        );
        assert_eq!(shim_nsprocessinfo_host_name(0, 0), 0);
        assert_eq!(shim_nsprocessinfo_process_name(0, 0), 0);
        assert_eq!(shim_nsprocessinfo_arguments(0, 0), 0);
    }

    // ── New selector tests ─────────────────────────────────────────

    #[test]
    fn globally_unique_string_returns_nonempty() {
        let s = shim_nsprocessinfo_globally_unique_string(1, 0);
        assert_ne!(s, 0);
        let text = read_string(s);
        assert!(!text.is_empty());
        assert!(text.contains('_'), "expected PID_timestamp_counter format");
    }

    #[test]
    fn globally_unique_string_is_unique() {
        let a = read_string(shim_nsprocessinfo_globally_unique_string(1, 0));
        let b = read_string(shim_nsprocessinfo_globally_unique_string(1, 0));
        assert_ne!(a, b, "two calls must produce different strings");
    }

    #[test]
    fn active_processor_count_at_least_one() {
        let count = shim_nsprocessinfo_active_processor_count(1, 0);
        assert!(count >= 1, "expected at least 1 processor, got {count}");
    }

    #[test]
    fn physical_memory_at_least_1gb() {
        let mem = shim_nsprocessinfo_physical_memory(1, 0);
        assert!(
            mem >= 1_073_741_824,
            "expected at least 1GB physical memory, got {mem}"
        );
    }

    #[test]
    fn system_uptime_positive() {
        let uptime = shim_nsprocessinfo_system_uptime(1, 0);
        assert!(uptime > 0.0, "expected positive system uptime");
    }
}
