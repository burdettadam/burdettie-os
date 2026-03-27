//! NSBundle shim: application bundle access.
//!
//! Object layout (heap-allocated, singleton for mainBundle):
//!
//!   offset  0: isa         (u64 — pointer to NSBundle class)
//!   offset  8: bundle_path (u64 — pointer to NSString with bundle path)
//!
//! Total: 16 bytes per NSBundle object.

use std::sync::OnceLock;

const NSBUNDLE_PATH_OFF: usize = 8;

static MAIN_BUNDLE: OnceLock<u64> = OnceLock::new();

fn get_or_create_main_bundle() -> u64 {
    *MAIN_BUNDLE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        if obj == 0 {
            return 0;
        }

        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsbundle_class_addr();
        }

        // Use the executable path from shim config, derive bundle dir
        let exe_path = libsystem_shims::process::process_path()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/usr/local/bin/unknown".to_string());

        // Detect .app bundle layout: Foo.app/Contents/MacOS/Foo
        // Walk up from the executable to find the .app root.
        let bundle_path = detect_app_bundle_root(&exe_path)
            .unwrap_or_else(|| {
                // Flat CLI: bundle path = directory containing the executable
                std::path::Path::new(&exe_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string())
            });

        let cstr = std::ffi::CString::new(bundle_path).unwrap_or_default();
        let nsstr = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
        unsafe {
            *((obj as usize + NSBUNDLE_PATH_OFF) as *mut u64) = nsstr;
        }

        obj
    })
}

/// Walk up from an executable path to detect a .app bundle root.
///
/// Standard layout: `Foo.app/Contents/MacOS/Foo` — we check if the path
/// walks up through `Contents/MacOS` inside a `*.app` directory.
fn detect_app_bundle_root(exe_path: &str) -> Option<String> {
    let path = std::path::Path::new(exe_path);
    // exe is at: .../Foo.app/Contents/MacOS/Foo
    let macos_dir = path.parent()?;                     // .../Foo.app/Contents/MacOS
    if macos_dir.file_name()?.to_str()? != "MacOS" {
        return None;
    }
    let contents_dir = macos_dir.parent()?;             // .../Foo.app/Contents
    if contents_dir.file_name()?.to_str()? != "Contents" {
        return None;
    }
    let app_dir = contents_dir.parent()?;               // .../Foo.app
    let name = app_dir.file_name()?.to_str()?;
    if name.ends_with(".app") {
        Some(app_dir.to_string_lossy().to_string())
    } else {
        None
    }
}

/// `+[NSBundle mainBundle]` — returns the main bundle singleton.
pub extern "C" fn shim_nsbundle_main_bundle(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_main_bundle()
}

/// `-[NSBundle bundlePath]` — returns the bundle directory path.
pub extern "C" fn shim_nsbundle_bundle_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) }
}

/// `-[NSBundle resourcePath]` — returns bundlePath + "/Contents/Resources"
/// (for a standard .app bundle), or just bundlePath for flat CLI bundles.
pub extern "C" fn shim_nsbundle_resource_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path_ptr = unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) };
    if path_ptr == 0 {
        return 0;
    }

    // Read the bundle path string
    let utf8 = crate::cfstring::shim_nsstring_utf8string(path_ptr, 0);
    if utf8.is_null() {
        return path_ptr;
    }

    let path_str = unsafe { std::ffi::CStr::from_ptr(utf8) }
        .to_str()
        .unwrap_or("");

    // For .app bundles, Contents/Resources exists; for flat CLIs, just return bundlePath
    let resources = std::path::Path::new(path_str).join("Contents/Resources");
    if resources.exists() {
        let cstr = std::ffi::CString::new(resources.to_string_lossy().as_ref()).unwrap_or_default();
        crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
    } else {
        path_ptr
    }
}

/// `-[NSBundle executablePath]` — returns the executable path.
pub extern "C" fn shim_nsbundle_executable_path(_obj: u64, _cmd: u64) -> u64 {
    let exe_path = libsystem_shims::process::process_path()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "/usr/local/bin/unknown".to_string());

    let cstr = std::ffi::CString::new(exe_path).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSBundle bundleIdentifier]` — returns nil for CLI tools.
pub extern "C" fn shim_nsbundle_bundle_identifier(_obj: u64, _cmd: u64) -> u64 {
    0 // nil — CLI tools typically don't have a bundle identifier
}
