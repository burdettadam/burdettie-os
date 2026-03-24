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

fn get_or_create_instance() -> u64 {
    *PROCESS_INFO_INSTANCE.get_or_init(|| {
        let obj = unsafe { libc::calloc(1, 8) } as u64;
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

/// `-[NSProcessInfo arguments]` — returns argv as NSArray of NSStrings.
pub extern "C" fn shim_nsprocessinfo_arguments(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    // Build a minimal NSArray with the process name as argv[0]
    let name = libsystem_shims::process::process_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "darwin-loader-lab".to_string());

    let cstr = std::ffi::CString::new(name).unwrap_or_default();
    let nsstr = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());

    // Allocate a one-element NSArray
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsarray_class_addr();
    }
    unsafe {
        *((obj as usize + 8) as *mut u64) = 1;
    } // count = 1

    let buf = unsafe { libc::malloc(8) } as u64;
    if buf != 0 {
        unsafe {
            *(buf as *mut u64) = nsstr;
        }
    }
    unsafe {
        *((obj as usize + 16) as *mut u64) = buf;
    }

    obj
}

/// `-[NSProcessInfo environment]` — returns NSDictionary of env vars.
///
/// Returns an empty dictionary for now (environment is sandboxed in the emulator).
pub extern "C" fn shim_nsprocessinfo_environment(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    // Create an empty NSDictionary (count=0)
    let dict = unsafe { libc::calloc(1, 40) } as u64;
    if dict == 0 {
        return 0;
    }
    unsafe {
        *(dict as *mut u64) = crate::nsobj::nsdictionary_class_addr();
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
