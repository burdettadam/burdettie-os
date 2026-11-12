// Foundation string constants — exported as stable pointer-identity symbols.
//
// Apple binaries reference these as extern `NSString * const` data symbols.
// We allocate each once via `OnceLock` so all pointer comparisons work.

use std::sync::OnceLock;

macro_rules! def_const {
    ($static:ident, $fn:ident, $value:expr) => {
        static $static: OnceLock<u64> = OnceLock::new();
        pub fn $fn() -> u64 {
            *$static.get_or_init(|| {
                let cstr = std::ffi::CString::new($value).unwrap();
                crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
            })
        }
    };
}

/// Data-symbol variant: returns the ADDRESS of a stable u64 cell that
/// holds the CFString pointer.  Used for extern `NSString * const` data
/// symbols imported via GOT, where the binary does:
///   ldr x0, [GOT]   // → address of the cell
///   ldr x1, [x0]    // → CFString pointer value
macro_rules! def_data_symbol {
    ($cell:ident, $init:ident, $fn:ident, $value:expr) => {
        static $cell: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        static $init: std::sync::Once = std::sync::Once::new();
        pub fn $fn() -> u64 {
            $init.call_once(|| {
                let cstr = std::ffi::CString::new($value).unwrap();
                let cfstr = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
                $cell.store(cfstr, std::sync::atomic::Ordering::Release);
            });
            // Return the address of the cell — binary dereferences to get the CFString pointer
            &$cell as *const std::sync::atomic::AtomicU64 as u64
        }
    };
}

// ── NSStream ───────────────────────────────────────────────────────
def_const!(
    STREAM_DATA_KEY,
    nsstream_data_written_to_memory_key_addr,
    "NSStreamDataWrittenToMemoryStreamKey"
);

// ── NSError domains ────────────────────────────────────────────────
def_const!(
    COCOA_DOMAIN,
    nscocoa_error_domain_addr,
    "NSCocoaErrorDomain"
);
def_const!(
    POSIX_DOMAIN,
    nsposix_error_domain_addr,
    "NSPOSIXErrorDomain"
);
def_const!(URL_DOMAIN, nsurl_error_domain_addr, "NSURLErrorDomain");
def_const!(MACH_DOMAIN, nsmach_error_domain_addr, "NSMachErrorDomain");

// ── NSError userInfo keys ──────────────────────────────────────────
def_const!(
    LOC_DESC_KEY,
    nslocalized_description_key_addr,
    "NSLocalizedDescription"
);
def_const!(
    LOC_REASON_KEY,
    nslocalized_failure_reason_key_addr,
    "NSLocalizedFailureReason"
);
def_const!(FILE_PATH_KEY, nsfile_path_error_key_addr, "NSFilePath");

// ── CFBundle keys ──────────────────────────────────────────────────
def_const!(
    CFBUNDLE_SHORT_VERSION_KEY,
    cfbundle_short_version_string_key_addr,
    "CFBundleShortVersionString"
);
def_const!(
    CFBUNDLE_VERSION_KEY,
    cfbundle_version_key_addr,
    "CFBundleVersion"
);

// ── NSNotification names ───────────────────────────────────────────
def_const!(
    UD_DID_CHANGE,
    nsuserdefaults_did_change_notification_addr,
    "NSUserDefaultsDidChangeNotification"
);

// ── NSAttributedString attribute keys ──────────────────────────────
def_const!(NSFONT_ATTR, nsfont_attribute_name_addr, "NSFont");
def_const!(
    NSFOREGROUND_COLOR_ATTR,
    nsforeground_color_attribute_name_addr,
    "NSColor"
);
def_const!(
    NSBACKGROUND_COLOR_ATTR,
    nsbackground_color_attribute_name_addr,
    "NSBackgroundColor"
);
def_const!(
    NSPARAGRAPH_STYLE_ATTR,
    nsparagraph_style_attribute_name_addr,
    "NSParagraphStyle"
);
def_const!(NSKERN_ATTR, nskern_attribute_name_addr, "NSKern");
def_const!(
    NSSTRIKETHROUGH_STYLE_ATTR,
    nsstrikethrough_style_attribute_name_addr,
    "NSStrikethroughStyle"
);
def_const!(
    NSUNDERLINE_STYLE_ATTR,
    nsunderline_style_attribute_name_addr,
    "NSUnderlineStyle"
);
def_const!(NSSHADOW_ATTR, nsshadow_attribute_name_addr, "NSShadow");
def_const!(NSLINK_ATTR, nslink_attribute_name_addr, "NSLink");
def_const!(
    NSBASELINE_OFFSET_ATTR,
    nsbaseline_offset_attribute_name_addr,
    "NSBaselineOffset"
);
def_const!(
    NSATTACHMENT_ATTR,
    nsattachment_attribute_name_addr,
    "NSAttachment"
);
def_const!(
    NSSUPERSCRIPT_ATTR,
    nssuperscript_attribute_name_addr,
    "NSSuperScript"
);
def_const!(
    NSSTRIKETHROUGH_COLOR_ATTR,
    nsstrikethrough_color_attribute_name_addr,
    "NSStrikethroughColor"
);
def_const!(
    NSUNDERLINE_COLOR_ATTR,
    nsunderline_color_attribute_name_addr,
    "NSUnderlineColor"
);

// ── NSAccessibility constants ──────────────────────────────────────
def_const!(
    NSACCESSIBILITY_ROLE,
    nsaccessibility_role_attribute_addr,
    "AXRole"
);
def_const!(
    NSACCESSIBILITY_ROLE_DESC,
    nsaccessibility_role_description_attribute_addr,
    "AXRoleDescription"
);
def_const!(
    NSACCESSIBILITY_TITLE,
    nsaccessibility_title_attribute_addr,
    "AXTitle"
);
def_const!(
    NSACCESSIBILITY_VALUE,
    nsaccessibility_value_attribute_addr,
    "AXValue"
);

// ── AppKit version ─────────────────────────────────────────────────

// ── SystemVersion keys (sw_vers / _CFCopySupplementalVersionDictionary) ──
// These are `extern CFStringRef const` data symbols — the binary loads the
// GOT slot to get the variable address, then dereferences to get the value.
def_data_symbol!(
    KCF_SYS_VER_PN_CELL,
    KCF_SYS_VER_PN_INIT,
    kcf_system_version_product_name_key_addr,
    "ProductName"
);
def_data_symbol!(
    KCF_SYS_VER_PV_CELL,
    KCF_SYS_VER_PV_INIT,
    kcf_system_version_product_version_key_addr,
    "ProductVersion"
);
def_data_symbol!(
    KCF_SYS_VER_BV_CELL,
    KCF_SYS_VER_BV_INIT,
    kcf_system_version_build_version_key_addr,
    "ProductBuildVersion"
);
def_data_symbol!(
    KCF_SYS_VER_PVE_CELL,
    KCF_SYS_VER_PVE_INIT,
    kcf_system_version_product_version_extra_key_addr,
    "ProductVersionExtra"
);

/// `NSAppKitVersionNumber` — exposed as a double data symbol (not a string).
static NSAPPKIT_VERSION_NUMBER: OnceLock<u64> = OnceLock::new();
pub fn nsappkit_version_number_addr() -> u64 {
    *NSAPPKIT_VERSION_NUMBER.get_or_init(|| {
        let addr = crate::pal_alloc().calloc(1, 8) as *mut f64;
        unsafe {
            // 2487.0 corresponds to macOS 14 Sonoma AppKit
            *addr = 2487.0;
        }
        addr as u64
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfbundle_short_version_key_is_stable_string() {
        let a = cfbundle_short_version_string_key_addr();
        let b = cfbundle_short_version_string_key_addr();
        assert_eq!(a, b);
        assert_eq!(
            crate::cfstring::extract_cfstring_cstr(a),
            Some("CFBundleShortVersionString")
        );
    }

    #[test]
    fn cfbundle_version_key_is_stable_string() {
        let a = cfbundle_version_key_addr();
        let b = cfbundle_version_key_addr();
        assert_eq!(a, b);
        assert_eq!(
            crate::cfstring::extract_cfstring_cstr(a),
            Some("CFBundleVersion")
        );
    }
}
