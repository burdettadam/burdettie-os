//! PKBundleComponentVersion shim: lightweight Xcode/PackageKit version wrapper.
//!
//! Object layout: 24 bytes
//!   [0..8): ISA pointer
//!   [8..16): currentMarketingVersion (NSString *)
//!   [16..24): bundleVersion (NSString *)

use crate::cfstring;
use crate::nsobj;

const MARKETING_VERSION_OFF: usize = 8;
const BUNDLE_VERSION_OFF: usize = 16;

fn nsstring_from_str(value: &str) -> u64 {
    let cstr = std::ffi::CString::new(value).unwrap_or_default();
    cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

fn xcode_bundle_root() -> Option<std::path::PathBuf> {
    let process_path = libsystem_shims::process::process_path()?;
    crate::nsbundle::detect_app_bundle_root(process_path).map(std::path::PathBuf::from)
}

fn xcode_plist_value(key: &str) -> u64 {
    let Some(bundle_root) = xcode_bundle_root() else {
        return 0;
    };

    let candidate_paths = [
        bundle_root.join("Contents/version.plist"),
        bundle_root.join("Contents/Info.plist"),
        bundle_root.join("Info.plist"),
    ];

    for path in candidate_paths {
        let Some(path_str) = path.to_str() else {
            continue;
        };
        let dict = crate::nspropertylistserialization::parse_plist_file(path_str);
        if dict == 0 {
            continue;
        }
        let key_obj = nsstring_from_str(key);
        if key_obj == 0 {
            return 0;
        }
        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key_obj);
        if value != 0 {
            return value;
        }
    }

    0
}

fn alloc_component_version(marketing_version: u64, bundle_version: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = nsobj::pkbundlecomponentversion_class_addr();
        *((obj as usize + MARKETING_VERSION_OFF) as *mut u64) = marketing_version;
        *((obj as usize + BUNDLE_VERSION_OFF) as *mut u64) = bundle_version;
    }

    obj
}

/// `+[PKBundleComponentVersion bundleComponentVersionWithOnDiskCFBundle:]`
pub extern "C" fn shim_pkbundlecomponentversion_with_on_disk_cfbundle(
    _cls: u64,
    _cmd: u64,
    _bundle: u64,
) -> u64 {
    let marketing_version = xcode_plist_value("CFBundleShortVersionString");
    let bundle_version = xcode_plist_value("CFBundleVersion");
    if marketing_version == 0 && bundle_version == 0 {
        return 0;
    }
    alloc_component_version(marketing_version, bundle_version)
}

/// `-[PKBundleComponentVersion currentMarketingVersion]`
pub extern "C" fn shim_pkbundlecomponentversion_current_marketing_version(
    obj: u64,
    _cmd: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + MARKETING_VERSION_OFF) as *const u64) }
}

/// `-[PKBundleComponentVersion bundleVersion]`
pub extern "C" fn shim_pkbundlecomponentversion_bundle_version(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + BUNDLE_VERSION_OFF) as *const u64) }
}

/// `-[PKBundleComponentVersion description]`
pub extern "C" fn shim_pkbundlecomponentversion_description(obj: u64, _cmd: u64) -> u64 {
    shim_pkbundlecomponentversion_current_marketing_version(obj, _cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn description_returns_marketing_version() {
        let marketing = make_string("26.4");
        let bundle = make_string("24909");
        let obj = alloc_component_version(marketing, bundle);
        assert_ne!(obj, 0);
        assert_string_eq(shim_pkbundlecomponentversion_description(obj, 0), "26.4");
        assert_string_eq(
            shim_pkbundlecomponentversion_current_marketing_version(obj, 0),
            "26.4",
        );
        assert_string_eq(shim_pkbundlecomponentversion_bundle_version(obj, 0), "24909");
        free_obj(obj);
    }
}