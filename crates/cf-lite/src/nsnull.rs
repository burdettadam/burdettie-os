//! NSNull shim: singleton null placeholder for collections.
//!
//! Object layout: 8 bytes (ISA pointer only — singleton).
//!
//! NSNull is used as a placeholder for nil in NSArray/NSDictionary since
//! these collections cannot store nil directly.

use std::sync::OnceLock;

use crate::cfstring;
use crate::nsobj;

static NULL_SINGLETON: OnceLock<u64> = OnceLock::new();

fn get_singleton() -> u64 {
    *NULL_SINGLETON.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        unsafe {
            *(obj as *mut u64) = nsobj::nsnull_class_addr();
        }
        obj
    })
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSNull null]` — returns the singleton NSNull instance.
pub extern "C" fn shim_nsnull_null(_cls: u64, _cmd: u64) -> u64 {
    get_singleton()
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSNull description]` — returns `"<null>"`.
pub extern "C" fn shim_nsnull_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let c_str = c"<null>";
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
