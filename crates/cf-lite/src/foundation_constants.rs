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

// ── NSStream ───────────────────────────────────────────────────────
def_const!(STREAM_DATA_KEY, nsstream_data_written_to_memory_key_addr, "NSStreamDataWrittenToMemoryStreamKey");

// ── NSError domains ────────────────────────────────────────────────
def_const!(COCOA_DOMAIN, nscocoa_error_domain_addr, "NSCocoaErrorDomain");
def_const!(POSIX_DOMAIN, nsposix_error_domain_addr, "NSPOSIXErrorDomain");
def_const!(URL_DOMAIN, nsurl_error_domain_addr, "NSURLErrorDomain");
def_const!(MACH_DOMAIN, nsmach_error_domain_addr, "NSMachErrorDomain");

// ── NSError userInfo keys ──────────────────────────────────────────
def_const!(LOC_DESC_KEY, nslocalized_description_key_addr, "NSLocalizedDescription");
def_const!(LOC_REASON_KEY, nslocalized_failure_reason_key_addr, "NSLocalizedFailureReason");
def_const!(FILE_PATH_KEY, nsfile_path_error_key_addr, "NSFilePath");

// ── CFBundle keys ──────────────────────────────────────────────────
def_const!(CFBUNDLE_SHORT_VERSION_KEY, cfbundle_short_version_string_key_addr, "CFBundleShortVersionString");
def_const!(CFBUNDLE_VERSION_KEY, cfbundle_version_key_addr, "CFBundleVersion");

// ── NSNotification names ───────────────────────────────────────────
def_const!(UD_DID_CHANGE, nsuserdefaults_did_change_notification_addr, "NSUserDefaultsDidChangeNotification");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfbundle_short_version_key_is_stable_string() {
        let a = cfbundle_short_version_string_key_addr();
        let b = cfbundle_short_version_string_key_addr();
        assert_eq!(a, b);
        assert_eq!(crate::cfstring::extract_cfstring_cstr(a), Some("CFBundleShortVersionString"));
    }

    #[test]
    fn cfbundle_version_key_is_stable_string() {
        let a = cfbundle_version_key_addr();
        let b = cfbundle_version_key_addr();
        assert_eq!(a, b);
        assert_eq!(crate::cfstring::extract_cfstring_cstr(a), Some("CFBundleVersion"));
    }
}
