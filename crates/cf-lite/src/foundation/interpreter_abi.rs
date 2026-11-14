//! Explicit host signatures for narrow interpreter adapters.

use super::FoundationShims;
use crate::cfstring::{shim_nsstring_range_of_character_from_set, NSRange};

impl FoundationShims {
    /// Call the known `rangeOfCharacterFromSet:` implementation with its real
    /// three-integer-argument, two-word-result signature.
    ///
    /// The exact resolved IMP address selects this adapter. A selector name,
    /// guest-supplied type encoding or another method returning a range does
    /// not select it. Unknown IMPs return `None` without calling or dereferencing
    /// anything. This does not translate variadic, FP or arbitrary struct ABIs.
    ///
    /// # Safety
    ///
    /// If `imp` is this known shim, `receiver` and `argument` must be valid
    /// NSString and NSCharacterSet objects (or null) for the duration of the
    /// call. Their backing memory must satisfy the shim's lifetime and aliasing
    /// requirements. The caller must first exclude guest-code addresses.
    pub unsafe fn call_known_nsrange_imp(
        imp: u64,
        receiver: u64,
        selector: u64,
        argument: u64,
    ) -> Option<(u64, u64)> {
        // This declaration binds the adapter to the implementation's actual
        // signature at compile time. Do not cast an arbitrary IMP to this type.
        let function: extern "C" fn(u64, u64, u64) -> NSRange =
            shim_nsstring_range_of_character_from_set;
        if imp != function as *const () as u64 {
            return None;
        }
        let range = function(receiver, selector, argument);
        Some((range.location, range.length))
    }
}
