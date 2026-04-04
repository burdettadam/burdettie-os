//! NSPredicate shim: predicate evaluation for filtered collections.
//!
//! Object layout (32 bytes):
//!   offset  0: isa           (u64)
//!   offset  8: format_ptr    (u64 → NSString)
//!   offset 16: predicate_fn  (u64 → leaked Box<dyn Fn(u64) -> bool>)
//!   offset 24: reserved      (u64)
//!
//! Supports keypath-based predicates evaluated against NSDictionary objects:
//!   - `keypath > N`, `keypath >= N`, `keypath < N`, `keypath <= N`
//!   - `keypath == %@`, `keypath == 'value'`
//!   - `keypath BEGINSWITH %@`, `keypath CONTAINS %@`, `keypath CONTAINS[c] %@`
//!   - `keypath LIKE 'pattern'`, `keypath LIKE[c] 'pattern'` (`*` = any, `?` = single char)
//!   - `keypath MATCHES 'regex'` (regex matching via `regex` crate)
//!   - `keypath IN %@`, `keypath BETWEEN %@`
//!   - Compound: `expr1 AND expr2`, `expr1 OR expr2`
//!   - `NOT expr`, `!(expr)`
//!   - `TRUEPREDICATE`, `FALSEPREDICATE`
//!   - `SELF == 'value'`, `SELF CONTAINS 'value'`, `SELF LIKE 'pattern'`, `SELF MATCHES 'regex'`

use crate::cfstring;

use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 32;

fn make_predicate(format: u64, pred_fn: Box<dyn Fn(u64) -> bool + Send + Sync>) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nspredicate_class_addr();
        *((obj + 8) as *mut u64) = format;
        *((obj + 16) as *mut u64) = Box::into_raw(Box::new(pred_fn)) as u64;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────────

/// ARM64 Apple wrapper: variadic args live on the stack, not in registers.
/// This naked function loads the first variadic arg from [sp] into x3
/// before tail-calling the real implementation.
#[cfg(target_arch = "aarch64")]
#[unsafe(naked)]
pub unsafe extern "C" fn shim_predicate_with_format() {
    core::arch::naked_asm!(
        "ldr x3, [sp]",
        "b {inner}",
        inner = sym shim_predicate_with_format_inner,
    );
}

#[cfg(not(target_arch = "aarch64"))]
pub extern "C" fn shim_predicate_with_format(
    _cls: u64, _cmd: u64, format: u64, arg0: u64,
) -> u64 {
    shim_predicate_with_format_inner(_cls, _cmd, format, arg0)
}

/// `+[NSPredicate predicateWithFormat:]` — inner implementation.
pub extern "C" fn shim_predicate_with_format_inner(
    _cls: u64,
    _cmd: u64,
    format: u64,
    arg0: u64,
) -> u64 {
    let fmt_str = match cfstring::extract_cfstring_cstr(format) {
        Some(s) => s.to_string(),
        None => return 0,
    };

    let predicate_fn = parse_format(&fmt_str, arg0);
    make_predicate(format, predicate_fn)
}

/// `+[NSPredicate predicateWithValue:]` — always-true or always-false.
pub extern "C" fn shim_predicate_with_value(_cls: u64, _cmd: u64, value: u8) -> u64 {
    let val = value != 0;
    make_predicate(0, Box::new(move |_| val))
}

/// `+[NSPredicate predicateWithBlock:]` — predicate backed by a block/closure.
///
/// The `block` argument is an ObjC block pointer whose invoke function has
/// signature `BOOL ^(id evaluatedObject, NSDictionary *bindings)`.  We store
/// the raw pointer and call it at evaluation time.  When the block is 0 (nil)
/// the predicate always returns NO.
pub extern "C" fn shim_predicate_with_block(_cls: u64, _cmd: u64, block: u64) -> u64 {
    if block == 0 {
        return make_predicate(0, Box::new(|_| false));
    }
    // ObjC block layout: the invoke function pointer is at offset 16.
    // Signature: BOOL (^)(id self_block, id evaluatedObject, NSDictionary *bindings)
    let invoke_ptr = unsafe { *((block as usize + 16) as *const u64) };
    if invoke_ptr == 0 {
        return make_predicate(0, Box::new(|_| false));
    }
    let blk = block;
    make_predicate(
        0,
        Box::new(move |target| {
            let f: extern "C" fn(u64, u64, u64) -> u8 =
                unsafe { std::mem::transmute(invoke_ptr) };
            f(blk, target, 0) != 0
        }),
    )
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSPredicate evaluateWithObject:]`
pub extern "C" fn shim_predicate_evaluate(obj: u64, _cmd: u64, target: u64) -> u8 {
    if obj == 0 { return 0; }
    let fn_ptr = unsafe { *((obj + 16) as *const u64) };
    if fn_ptr == 0 { return 0; }
    let predicate: &(dyn Fn(u64) -> bool + Send + Sync) =
        unsafe { &*(fn_ptr as *const Box<dyn Fn(u64) -> bool + Send + Sync>) };
    u8::from(predicate(target))
}

/// `-[NSPredicate predicateFormat]`
pub extern "C" fn shim_predicate_format(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSPredicate description]`
pub extern "C" fn shim_predicate_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let fmt = unsafe { *((obj + 8) as *const u64) };
    if fmt != 0 { return fmt; }
    let c_str = c"<NSPredicate>";
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}


mod parser;
use self::parser::parse_format;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn free_predicate(obj: u64) {
        // The closure stored at offset 16 is intentionally leaked in tests.
        free_obj(obj);
    }

    #[test]
    fn nil_evaluate_returns_zero() {
        assert_eq!(shim_predicate_evaluate(0, 0, 0), 0);
    }

    #[test]
    fn nil_format_returns_zero() {
        assert_eq!(shim_predicate_format(0, 0), 0);
    }

    #[test]
    fn nil_description_returns_zero() {
        assert_eq!(shim_predicate_description(0, 0), 0);
    }

    #[test]
    fn predicate_with_value_true_evaluates_true() {
        let pred = shim_predicate_with_value(0, 0, 1);
        assert_ne!(pred, 0);
        assert_eq!(shim_predicate_evaluate(pred, 0, 0), 1);
        free_predicate(pred);
    }

    #[test]
    fn predicate_with_value_false_evaluates_false() {
        let pred = shim_predicate_with_value(0, 0, 0);
        assert_ne!(pred, 0);
        assert_eq!(shim_predicate_evaluate(pred, 0, 0), 0);
        free_predicate(pred);
    }

    #[test]
    fn predicate_description_no_format_returns_sentinel() {
        let pred = shim_predicate_with_value(0, 0, 1);
        let desc = shim_predicate_description(pred, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSPredicate>");
        free_predicate(pred);
    }

    #[test]
    fn predicate_format_inner_truepredicate() {
        let fmt = make_string("TRUEPREDICATE");
        let pred = shim_predicate_with_format_inner(0, 0, fmt, 0);
        assert_ne!(pred, 0);
        assert_eq!(shim_predicate_evaluate(pred, 0, 0), 1);
        free_predicate(pred);
    }

    #[test]
    fn predicate_format_inner_falsepredicate() {
        let fmt = make_string("FALSEPREDICATE");
        let pred = shim_predicate_with_format_inner(0, 0, fmt, 0);
        assert_ne!(pred, 0);
        assert_eq!(shim_predicate_evaluate(pred, 0, 0), 0);
        free_predicate(pred);
    }

    #[test]
    fn predicate_with_block_nil_block_returns_false() {
        let pred = shim_predicate_with_block(0, 0, 0);
        assert_ne!(pred, 0);
        assert_eq!(shim_predicate_evaluate(pred, 0, 42), 0);
        free_predicate(pred);
    }

    #[test]
    fn predicate_with_block_always_true() {
        // Build a fake ObjC block: [isa, flags, reserved, invoke, descriptor]
        // invoke at offset 16 returns 1 for any input.
        extern "C" fn always_true(_blk: u64, _obj: u64, _bindings: u64) -> u8 { 1 }
        let mut block = [0u64; 5];
        block[2] = always_true as *const () as u64; // offset 16 = invoke
        let pred = shim_predicate_with_block(0, 0, block.as_ptr() as u64);
        assert_ne!(pred, 0);
        assert_eq!(shim_predicate_evaluate(pred, 0, 0), 1);
        free_predicate(pred);
    }

    #[test]
    fn predicate_with_block_always_false() {
        extern "C" fn always_false(_blk: u64, _obj: u64, _bindings: u64) -> u8 { 0 }
        let mut block = [0u64; 5];
        block[2] = always_false as *const () as u64; // offset 16 = invoke
        let pred = shim_predicate_with_block(0, 0, block.as_ptr() as u64);
        assert_ne!(pred, 0);
        assert_eq!(shim_predicate_evaluate(pred, 0, 0), 0);
        free_predicate(pred);
    }
}
