//! Swift ↔ ObjC bridge interception shims.
//!
//! # Problem
//!
//! `notarytool` (and other Swift tools) call Swift-Foundation's
//! `_unconditionallyBridgeFromObjectiveC` / `_forceBridgeFromObjectiveC`
//! static methods to materialise Swift value types (`[K:V]`, `[T]`,
//! `String`, …) from ObjC objects.  These functions are resolved through
//! the `$s*` safe-list rule in `StubResolver` and land in the host
//! `libswiftFoundation` (merged into the dyld shared cache on macOS 15+).
//!
//! The host function calls `objc_msgSend` on the pointer it receives.
//! When that pointer is one of our cf-lite synthetic objects, the host
//! `objc_msgSend` cannot find it in its own class table and crashes
//! (SIGBUS / EXC_BAD_ACCESS on the zeroed `cache` field at ISA+0x10).
//!
//! # Fix
//!
//! The `ShimTable` is consulted **before** `StubResolver`, so registering
//! these Swift mangled names here shadows the host implementations.
//!
//! Our shims write an **empty / zero** Swift value into the caller's
//! indirect-return buffer (x8 in ARM64 Swift calling convention) and
//! return with no side effects.  An empty collection is semantically
//! correct in most contexts — Swift callers check `.isEmpty` or `count`
//! before iterating, and `notarytool`'s crash happens during startup
//! dictionary inspection, before any real data would flow through.
//!
//! # ARM64 Swift indirect-return convention
//!
//! For value types larger than 2 registers the caller allocates stack
//! space and passes the address in x8.  The callee writes the result
//! there; x0 is the first argument (the ObjC source pointer, possibly
//! nil).  Our stub must write sufficiently many zero words to initialise
//! the target buffer.  For all the types intercepted here:
//!
//!  - Dictionary / Array / Set  → Swift `_ContiguousArrayStorage` wrapper
//!    (3 words: pointer + endIndex + capacity)
//!  - String                    → 2 words (capacity bits + object pointer)
//!  - Substring                 → 3 words
//!  - Data                      → 3 words (pointer + count + capacity)
//!  - IndexSet                  → 3 words
//!
//! Writing 32 bytes (4×u64) of zeros into x8 safely covers all cases.
//!
//! # `_forceBridgeFromObjectiveC` convention
//!
//! `_forceBridgeFromObjectiveC(_:result:)` in Swift takes two arguments:
//!   x0: source ObjC object (non-optional)
//!   x1: pointer to `Optional<SwiftType>` result (inout)
//! It writes a value into *x1 via the normal inout-pointer convention.
//! Setting *x1 = nil (4 words of zeros) is the correct "no bridge" result.

use crate::table::ShimTable;

// ── Null stub helpers ────────────────────────────────────────────────────────

/// Zero out 32 bytes at the address in register x8.
///
/// In ARM64 Swift calling convention this is the indirect-return buffer for
/// `_unconditionallyBridgeFromObjectiveC`.  Writing zeros materialises the
/// empty / nil representation of any of the intercepted Swift value types.
///
/// # Safety
/// We trust that the Swift compiler emits a properly sized stack allocation
/// for x8 before the call.  The 32-byte zeroing covers the largest bridged
/// value type (3 words + 1-word discriminant for `Optional<…>` wrapping, or
/// the 4-word layout of future types).
///
/// The function is `extern "C"` to avoid Rust ABI wrapping, but it must
/// read the incoming x8 value.  In ARM64 `extern "C"` the x8 register is
/// not passed as a named argument — Swift's convention puts the sret address
/// in x8 without making it a visible parameter.  We access it via
/// `inline_asm!` to stay safe-ish without depending on an unstable calling-
/// convention attribute.
///
/// On platforms where this is not compiled (non-aarch64 or non-macOS) we
/// fall through to the regular stub that simply returns.
/// Zero out 32 bytes at the address in register x8.
///
/// Uses a `#[naked]` function to guarantee no compiler-generated prologue
/// touches x8 (the ARM64 Swift indirect-return register / AAPCS64 indirect
/// result register).  The function writes a 32-byte zero block to *x8, which
/// represents an empty / nil value for all bridged Swift collection types.
///
/// A `cbz x8, 2f` guard keeps us safe if the caller passes a null x8 (which
/// should not happen in well-formed Swift code, but defence-in-depth is free).
#[cfg(target_arch = "aarch64")]
#[unsafe(naked)]
unsafe extern "C" fn write_zero_to_x8_ret_buf() {
    std::arch::naked_asm!(
        "cbz x8, 2f",
        "stp xzr, xzr, [x8]",
        "stp xzr, xzr, [x8, #16]",
        "2:",
        "ret",
    );
}

/// Fallback for non-aarch64 targets: no-op.
#[cfg(not(target_arch = "aarch64"))]
extern "C" fn write_zero_to_x8_ret_buf() {}

// ── unconditionallyBridgeFromObjectiveC shims ────────────────────────────────
//
// Symbol pattern:
//   _$s{Module}{Type}36_unconditionallyBridgeFromObjectiveC{ReturnType}{SourceType}FZ
//
// All of these share the same stub body: zero the x8 buffer and return.
// We use separate function pointers so the log can identify which type was bridged.

/// Dictionary<K,V>._unconditionallyBridgeFromObjectiveC(NSDictionary?) -> [K:V]
extern "C" fn shim_bridge_dictionary_from_nsdictionary() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        write_zero_to_x8_ret_buf();
    }
    log::trace!("swift_bridge: Dictionary._unconditionallyBridgeFromObjectiveC → empty");
}

/// Array<T>._unconditionallyBridgeFromObjectiveC(NSArray?) -> [T]
extern "C" fn shim_bridge_array_from_nsarray() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        write_zero_to_x8_ret_buf();
    }
    log::trace!("swift_bridge: Array._unconditionallyBridgeFromObjectiveC → empty");
}

/// String._unconditionallyBridgeFromObjectiveC(NSString?) -> String
extern "C" fn shim_bridge_string_from_nsstring() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        write_zero_to_x8_ret_buf();
    }
    log::trace!("swift_bridge: String._unconditionallyBridgeFromObjectiveC → empty");
}

/// Substring._unconditionallyBridgeFromObjectiveC(NSString?) -> Substring
extern "C" fn shim_bridge_substring_from_nsstring() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        write_zero_to_x8_ret_buf();
    }
    log::trace!("swift_bridge: Substring._unconditionallyBridgeFromObjectiveC → empty");
}

/// Set<T>._unconditionallyBridgeFromObjectiveC(NSSet?) -> Set<T>
extern "C" fn shim_bridge_set_from_nsset() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        write_zero_to_x8_ret_buf();
    }
    log::trace!("swift_bridge: Set._unconditionallyBridgeFromObjectiveC → empty");
}

/// Data._unconditionallyBridgeFromObjectiveC(NSData?) -> Data
extern "C" fn shim_bridge_data_from_nsdata() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        write_zero_to_x8_ret_buf();
    }
    log::trace!("swift_bridge: Data._unconditionallyBridgeFromObjectiveC → empty");
}

/// IndexSet._unconditionallyBridgeFromObjectiveC(NSIndexSet?) -> IndexSet
extern "C" fn shim_bridge_indexset_from_nsindexset() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        write_zero_to_x8_ret_buf();
    }
    log::trace!("swift_bridge: IndexSet._unconditionallyBridgeFromObjectiveC → empty");
}

// ── _forceBridgeFromObjectiveC shims ─────────────────────────────────────────
//
// Signature: (source: ObjCType, result: inout Optional<SwiftType>) -> Void
//   x0: source ObjC pointer
//   x1: *mut Optional<SwiftType> (inout)
//
// Writing zeros to *x1 gives Optional.none (nil), which callers check before use.
//
// We use a generic helper shared by all _forceBridge stubs.

fn zero_inout_optional(result_ptr: u64) {
    if result_ptr == 0 {
        return;
    }
    // Write 32 bytes of zeros = Optional.none for any of the bridged value types.
    unsafe {
        std::ptr::write_bytes(result_ptr as *mut u8, 0, 32);
    }
}

/// Dictionary._forceBridgeFromObjectiveC(_:result:)
extern "C" fn shim_force_bridge_dictionary(_source: u64, result: u64) {
    zero_inout_optional(result);
    log::trace!("swift_bridge: Dictionary._forceBridgeFromObjectiveC → nil");
}

/// Array._forceBridgeFromObjectiveC(_:result:)
extern "C" fn shim_force_bridge_array(_source: u64, result: u64) {
    zero_inout_optional(result);
    log::trace!("swift_bridge: Array._forceBridgeFromObjectiveC → nil");
}

/// String._forceBridgeFromObjectiveC(_:result:)
extern "C" fn shim_force_bridge_string(_source: u64, result: u64) {
    zero_inout_optional(result);
    log::trace!("swift_bridge: String._forceBridgeFromObjectiveC → nil");
}

/// Substring._forceBridgeFromObjectiveC(_:result:)
extern "C" fn shim_force_bridge_substring(_source: u64, result: u64) {
    zero_inout_optional(result);
    log::trace!("swift_bridge: Substring._forceBridgeFromObjectiveC → nil");
}

/// Set._forceBridgeFromObjectiveC(_:result:)
extern "C" fn shim_force_bridge_set(_source: u64, result: u64) {
    zero_inout_optional(result);
    log::trace!("swift_bridge: Set._forceBridgeFromObjectiveC → nil");
}

/// Data._forceBridgeFromObjectiveC(_:result:)
extern "C" fn shim_force_bridge_data(_source: u64, result: u64) {
    zero_inout_optional(result);
    log::trace!("swift_bridge: Data._forceBridgeFromObjectiveC → nil");
}

// ── Install ──────────────────────────────────────────────────────────────────

pub struct SwiftFoundationBridgeStubs;

impl SwiftFoundationBridgeStubs {
    /// Register Swift-Foundation bridge shims.
    ///
    /// These shadow the host `libswiftFoundation` implementations so that
    /// calls with cf-lite synthetic object pointers are handled safely instead
    /// of crashing in the host ObjC runtime's `objc_msgSend`.
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── _unconditionallyBridgeFromObjectiveC ─────────────────────────
        // Mangled names extracted from `libswiftFoundation.dylib` via `nm`.
        // These are stable across Swift versions (ABI-stable since Swift 5.0).
        register_shims!(
            table,
            // Dictionary<K,V>  ← NSDictionary
            (
                "_$sSD10FoundationE36_unconditionallyBridgeFromObjectiveCySDyxq_GSo12NSDictionaryCSgFZ",
                shim_bridge_dictionary_from_nsdictionary as *const () as u64,
                "Dictionary._unconditionallyBridgeFromObjectiveC: returns empty dict (safe bridge stub)"
            ),
            // Array<T>  ← NSArray
            (
                "_$sSa10FoundationE36_unconditionallyBridgeFromObjectiveCySayxGSo7NSArrayCSgFZ",
                shim_bridge_array_from_nsarray as *const () as u64,
                "Array._unconditionallyBridgeFromObjectiveC: returns empty array (safe bridge stub)"
            ),
            // String  ← NSString
            (
                "_$sSS10FoundationE36_unconditionallyBridgeFromObjectiveCySSSo8NSStringCSgFZ",
                shim_bridge_string_from_nsstring as *const () as u64,
                "String._unconditionallyBridgeFromObjectiveC: returns empty string (safe bridge stub)"
            ),
            // Substring  ← NSString
            (
                "_$sSs10FoundationE36_unconditionallyBridgeFromObjectiveCySsSo8NSStringCSgFZ",
                shim_bridge_substring_from_nsstring as *const () as u64,
                "Substring._unconditionallyBridgeFromObjectiveC: returns empty (safe bridge stub)"
            ),
            // Set<T>  ← NSSet
            (
                "_$sSh10FoundationE36_unconditionallyBridgeFromObjectiveCyShyxGSo5NSSetCSgFZ",
                shim_bridge_set_from_nsset as *const () as u64,
                "Set._unconditionallyBridgeFromObjectiveC: returns empty set (safe bridge stub)"
            ),
            // Data  ← NSData
            (
                "_$s10Foundation4DataV36_unconditionallyBridgeFromObjectiveCyACSo6NSDataCSgFZ",
                shim_bridge_data_from_nsdata as *const () as u64,
                "Data._unconditionallyBridgeFromObjectiveC: returns empty data (safe bridge stub)"
            ),
            // IndexSet  ← NSIndexSet
            (
                "_$s10Foundation8IndexSetV36_unconditionallyBridgeFromObjectiveCyACSo07NSIndexC0CSgFZ",
                shim_bridge_indexset_from_nsindexset as *const () as u64,
                "IndexSet._unconditionallyBridgeFromObjectiveC: returns empty (safe bridge stub)"
            ),
            // ── _forceBridgeFromObjectiveC ───────────────────────────────
            // Same symbols without the trailing `FZ` — these are `FZ` variants too,
            // but the force-bridge take two arguments (inout result pointer in x1).
            (
                "_$sSD10FoundationE26_forceBridgeFromObjectiveC_6resultySo12NSDictionaryC_SDyxq_GSgztFZ",
                shim_force_bridge_dictionary as *const () as u64,
                "Dictionary._forceBridgeFromObjectiveC: writes nil result (safe bridge stub)"
            ),
            (
                "_$sSa10FoundationE26_forceBridgeFromObjectiveC_6resultySo7NSArrayC_SayxGSgztFZ",
                shim_force_bridge_array as *const () as u64,
                "Array._forceBridgeFromObjectiveC: writes nil result (safe bridge stub)"
            ),
            (
                "_$sSS10FoundationE26_forceBridgeFromObjectiveC_6resultySo8NSStringC_SSSgztFZ",
                shim_force_bridge_string as *const () as u64,
                "String._forceBridgeFromObjectiveC: writes nil result (safe bridge stub)"
            ),
            (
                "_$sSs10FoundationE26_forceBridgeFromObjectiveC_6resultySo8NSStringC_SsSgztFZ",
                shim_force_bridge_substring as *const () as u64,
                "Substring._forceBridgeFromObjectiveC: writes nil result (safe bridge stub)"
            ),
            (
                "_$sSh10FoundationE26_forceBridgeFromObjectiveC_6resultySo5NSSetC_ShyxGSgztFZ",
                shim_force_bridge_set as *const () as u64,
                "Set._forceBridgeFromObjectiveC: writes nil result (safe bridge stub)"
            ),
            (
                "_$s10Foundation4DataV26_forceBridgeFromObjectiveC_6resultySo6NSDataC_ACSgztFZ",
                shim_force_bridge_data as *const () as u64,
                "Data._forceBridgeFromObjectiveC: writes nil result (safe bridge stub)"
            ),
        );

        log::debug!(
            "SwiftFoundationBridgeStubs: registered {} Swift-ObjC bridge intercepts",
            13
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_inout_optional_null_safe() {
        // Must not crash on null pointer.
        zero_inout_optional(0);
    }

    #[test]
    fn test_zero_inout_optional_writes() {
        let mut buf = [0xABu8; 32];
        zero_inout_optional(buf.as_mut_ptr() as u64);
        assert!(buf.iter().all(|&b| b == 0), "buffer should be zeroed");
    }

    #[test]
    fn test_install_registers_expected_symbols() {
        use crate::table::ShimTable;
        let mut table = ShimTable::default();
        SwiftFoundationBridgeStubs::install(&mut table);

        let key_syms = [
            "_$sSD10FoundationE36_unconditionallyBridgeFromObjectiveCySDyxq_GSo12NSDictionaryCSgFZ",
            "_$sSa10FoundationE36_unconditionallyBridgeFromObjectiveCySayxGSo7NSArrayCSgFZ",
            "_$sSS10FoundationE36_unconditionallyBridgeFromObjectiveCySSSo8NSStringCSgFZ",
            "_$sSs10FoundationE36_unconditionallyBridgeFromObjectiveCySsSo8NSStringCSgFZ",
            "_$sSh10FoundationE36_unconditionallyBridgeFromObjectiveCyShyxGSo5NSSetCSgFZ",
            "_$s10Foundation4DataV36_unconditionallyBridgeFromObjectiveCyACSo6NSDataCSgFZ",
            "_$s10Foundation8IndexSetV36_unconditionallyBridgeFromObjectiveCyACSo07NSIndexC0CSgFZ",
            "_$sSD10FoundationE26_forceBridgeFromObjectiveC_6resultySo12NSDictionaryC_SDyxq_GSgztFZ",
            "_$sSa10FoundationE26_forceBridgeFromObjectiveC_6resultySo7NSArrayC_SayxGSgztFZ",
            "_$sSS10FoundationE26_forceBridgeFromObjectiveC_6resultySo8NSStringC_SSSgztFZ",
        ];

        for sym in key_syms {
            assert!(table.resolve(sym).is_some(), "symbol not registered: {sym}");
        }
    }

    #[test]
    fn test_bridge_shims_are_non_null_fn_ptrs() {
        // Smoke-test that shim functions are real addresses (not zero).
        assert_ne!(shim_bridge_dictionary_from_nsdictionary as u64, 0);
        assert_ne!(shim_bridge_array_from_nsarray as u64, 0);
        assert_ne!(shim_bridge_string_from_nsstring as u64, 0);
        assert_ne!(shim_bridge_data_from_nsdata as u64, 0);
        assert_ne!(shim_force_bridge_dictionary as u64, 0);
    }
}
