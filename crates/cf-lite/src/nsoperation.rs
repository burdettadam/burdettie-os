//! NSOperationQueue / NSOperation / NSBlockOperation shims.
//!
//! Single-threaded implementations: operations execute synchronously
//! on the calling thread. This matches the lab's single-threaded model.
//!
//! NSOperation layout (24 bytes):
//!   offset  0: isa        (u64)
//!   offset  8: cancelled  (u64 → 0 or 1)
//!   offset 16: finished   (u64 → 0 or 1)
//!
//! NSBlockOperation layout (32 bytes):
//!   offset  0: isa        (u64)
//!   offset  8: cancelled  (u64)
//!   offset 16: finished   (u64)
//!   offset 24: block_ptr  (u64 → ObjC block)
//!
//! NSOperationQueue layout (32 bytes):
//!   offset  0: isa        (u64)
//!   offset  8: name_ptr   (u64 → NSString or 0)
//!   offset 16: suspended  (u64 → 0 or 1)
//!   offset 24: max_concurrent (u64 → default -1 = unlimited)

use crate::cfstring;
use crate::nsobj;

pub const OP_INSTANCE_SIZE: u32 = 24;
pub const BLOCK_OP_INSTANCE_SIZE: u32 = 32;
pub const QUEUE_INSTANCE_SIZE: u32 = 32;

// ── NSOperation ────────────────────────────────────────────────────

/// `+[NSOperation alloc]`
pub extern "C" fn shim_nsoperation_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, OP_INSTANCE_SIZE as usize) as u64;
    if obj != 0 {
        unsafe { *(obj as *mut u64) = nsobj::nsoperation_class_addr(); }
    }
    obj
}

/// `-[NSOperation init]`
pub extern "C" fn shim_nsoperation_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSOperation cancel]`
pub extern "C" fn shim_nsoperation_cancel(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    unsafe { *((obj + 8) as *mut u64) = 1; }
}

/// `-[NSOperation isCancelled]`
pub extern "C" fn shim_nsoperation_is_cancelled(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    (unsafe { *((obj + 8) as *const u64) } != 0) as u8
}

/// `-[NSOperation isFinished]`
pub extern "C" fn shim_nsoperation_is_finished(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    (unsafe { *((obj + 16) as *const u64) } != 0) as u8
}

/// `-[NSOperation main]` — subclasses override. Base does nothing.
pub extern "C" fn shim_nsoperation_main(_obj: u64, _cmd: u64) {}

/// `-[NSOperation start]` — executes `main` synchronously.
pub extern "C" fn shim_nsoperation_start(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let cancelled = unsafe { *((obj + 8) as *const u64) };
    if cancelled != 0 { return; }
    shim_nsoperation_main(obj, 0);
    unsafe { *((obj + 16) as *mut u64) = 1; } // finished
}

/// `-[NSOperation isReady]` — always returns YES (basic stub).
pub extern "C" fn shim_nsoperation_is_ready(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSOperation isAsynchronous]` — returns NO (synchronous execution).
pub extern "C" fn shim_nsoperation_is_asynchronous(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[NSOperation qualityOfService]` — returns NSQualityOfServiceDefault (-1).
pub extern "C" fn shim_nsoperation_quality_of_service(_obj: u64, _cmd: u64) -> i64 {
    -1
}

/// `-[NSOperation setQualityOfService:]` — no-op stub.
pub extern "C" fn shim_nsoperation_set_quality_of_service(_obj: u64, _cmd: u64, _qos: i64) {}

// ── NSBlockOperation ───────────────────────────────────────────────

/// `+[NSBlockOperation blockOperationWithBlock:]`
pub extern "C" fn shim_block_op_with_block(_cls: u64, _cmd: u64, block: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, BLOCK_OP_INSTANCE_SIZE as usize) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsblockoperation_class_addr();
        *((obj + 24) as *mut u64) = block;
    }
    obj
}

/// `-[NSBlockOperation start]` — invokes the block synchronously.
pub extern "C" fn shim_block_op_start(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let cancelled = unsafe { *((obj + 8) as *const u64) };
    if cancelled != 0 { return; }

    let block = unsafe { *((obj + 24) as *const u64) };
    if block != 0 {
        // Call the block's invoke function (offset 16 in ObjC block layout)
        let invoke: extern "C" fn(u64) = unsafe {
            std::mem::transmute(*((block + 16) as *const u64))
        };
        invoke(block);
    }
    unsafe { *((obj + 16) as *mut u64) = 1; } // finished
}

// ── NSOperationQueue ───────────────────────────────────────────────

/// `+[NSOperationQueue alloc]`
pub extern "C" fn shim_queue_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, QUEUE_INSTANCE_SIZE as usize) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = nsobj::nsoperationqueue_class_addr();
            // Default maxConcurrentOperationCount = -1 (unlimited)
            *((obj + 24) as *mut u64) = u64::MAX;
        }
    }
    obj
}

/// `-[NSOperationQueue init]`
pub extern "C" fn shim_queue_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSOperationQueue addOperation:]` — executes synchronously.
pub extern "C" fn shim_queue_add_operation(obj: u64, _cmd: u64, op: u64) {
    if obj == 0 || op == 0 { return; }
    let suspended = unsafe { *((obj + 16) as *const u64) };
    if suspended != 0 { return; }

    // Determine if it's a block operation or regular
    let isa = unsafe { *(op as *const u64) };
    if isa == nsobj::nsblockoperation_class_addr() {
        shim_block_op_start(op, 0);
    } else {
        shim_nsoperation_start(op, 0);
    }
}

/// `-[NSOperationQueue addOperationWithBlock:]` — convenience.
pub extern "C" fn shim_queue_add_block(obj: u64, _cmd: u64, block: u64) {
    let op = shim_block_op_with_block(0, 0, block);
    shim_queue_add_operation(obj, 0, op);
}

/// `-[NSOperationQueue setSuspended:]`
pub extern "C" fn shim_queue_set_suspended(obj: u64, _cmd: u64, suspended: u8) {
    if obj == 0 { return; }
    unsafe { *((obj + 16) as *mut u64) = u64::from(suspended != 0); }
}

/// `-[NSOperationQueue isSuspended]`
pub extern "C" fn shim_queue_is_suspended(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    (unsafe { *((obj + 16) as *const u64) } != 0) as u8
}

/// `-[NSOperationQueue setName:]`
pub extern "C" fn shim_queue_set_name(obj: u64, _cmd: u64, name: u64) {
    if obj == 0 { return; }
    unsafe { *((obj + 8) as *mut u64) = name; }
}

/// `-[NSOperationQueue name]`
pub extern "C" fn shim_queue_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSOperationQueue setMaxConcurrentOperationCount:]`
pub extern "C" fn shim_queue_set_max_concurrent(obj: u64, _cmd: u64, max: i64) {
    if obj == 0 { return; }
    unsafe { *((obj + 24) as *mut u64) = max as u64; }
}

/// `-[NSOperationQueue maxConcurrentOperationCount]`
pub extern "C" fn shim_queue_max_concurrent(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return -1; }
    unsafe { *((obj + 24) as *const u64) as i64 }
}

/// `-[NSOperationQueue description]`
pub extern "C" fn shim_queue_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let name = unsafe { *((obj + 8) as *const u64) };
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).unwrap_or("<unnamed>")
    } else {
        "<unnamed>"
    };
    let desc = format!("<NSOperationQueue: {name_str}>");
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}

/// `-[NSOperationQueue qualityOfService]` — returns NSQualityOfServiceDefault (-1).
pub extern "C" fn shim_queue_quality_of_service(_obj: u64, _cmd: u64) -> i64 {
    -1
}

/// `-[NSOperationQueue setQualityOfService:]` — no-op stub.
pub extern "C" fn shim_queue_set_quality_of_service(_obj: u64, _cmd: u64, _qos: i64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn operation_alloc_init_clean() {
        let op = shim_nsoperation_alloc(0, 0);
        assert_ne!(op, 0);
        assert_eq!(shim_nsoperation_init(op, 0), op);
        assert_eq!(shim_nsoperation_is_cancelled(op, 0), 0);
        assert_eq!(shim_nsoperation_is_finished(op, 0), 0);
        free_obj(op);
    }

    #[test]
    fn operation_nil_safety() {
        assert_eq!(shim_nsoperation_is_cancelled(0, 0), 0);
        assert_eq!(shim_nsoperation_is_finished(0, 0), 0);
        shim_nsoperation_cancel(0, 0);
        shim_nsoperation_start(0, 0);
    }

    #[test]
    fn operation_cancel_sets_flag() {
        let op = shim_nsoperation_alloc(0, 0);
        assert_eq!(shim_nsoperation_is_cancelled(op, 0), 0);
        shim_nsoperation_cancel(op, 0);
        assert_eq!(shim_nsoperation_is_cancelled(op, 0), 1);
        free_obj(op);
    }

    #[test]
    fn operation_start_sets_finished() {
        let op = shim_nsoperation_alloc(0, 0);
        shim_nsoperation_start(op, 0);
        assert_eq!(shim_nsoperation_is_finished(op, 0), 1);
        free_obj(op);
    }

    #[test]
    fn operation_start_when_cancelled_skips_execution() {
        let op = shim_nsoperation_alloc(0, 0);
        shim_nsoperation_cancel(op, 0);
        shim_nsoperation_start(op, 0);
        // start bails early if cancelled — finished remains 0
        assert_eq!(shim_nsoperation_is_finished(op, 0), 0);
        free_obj(op);
    }

    #[test]
    fn queue_alloc_init_and_suspend_round_trip() {
        let q = shim_queue_alloc(0, 0);
        assert_ne!(q, 0);
        assert_eq!(shim_queue_is_suspended(q, 0), 0);
        shim_queue_set_suspended(q, 0, 1);
        assert_eq!(shim_queue_is_suspended(q, 0), 1);
        shim_queue_set_suspended(q, 0, 0);
        assert_eq!(shim_queue_is_suspended(q, 0), 0);
        free_obj(q);
    }

    #[test]
    fn queue_add_operation_executes_synchronously() {
        let q = shim_queue_alloc(0, 0);
        let op = shim_nsoperation_alloc(0, 0);
        shim_queue_add_operation(q, 0, op);
        assert_eq!(shim_nsoperation_is_finished(op, 0), 1);
        free_obj(q);
        free_obj(op);
    }

    #[test]
    fn queue_name_round_trip() {
        let q = shim_queue_alloc(0, 0);
        assert_eq!(shim_queue_name(q, 0), 0);
        let name = make_string("my-queue");
        shim_queue_set_name(q, 0, name);
        assert_string_eq(shim_queue_name(q, 0), "my-queue");
        free_obj(q);
    }

    #[test]
    fn queue_description_unnamed() {
        let q = shim_queue_alloc(0, 0);
        let d = shim_queue_description(q, 0);
        assert_ne!(d, 0);
        assert_string_eq(d, "<NSOperationQueue: <unnamed>>");
        free_obj(q);
    }

    #[test]
    fn operation_is_ready_returns_true() {
        assert_eq!(shim_nsoperation_is_ready(0, 0), 1);
    }

    #[test]
    fn operation_is_asynchronous_returns_false() {
        assert_eq!(shim_nsoperation_is_asynchronous(0, 0), 0);
    }

    #[test]
    fn operation_quality_of_service_returns_default() {
        assert_eq!(shim_nsoperation_quality_of_service(0, 0), -1);
    }

    #[test]
    fn operation_set_quality_of_service_is_noop() {
        shim_nsoperation_set_quality_of_service(0, 0, 0x21);
        // no panic = pass
    }

    #[test]
    fn queue_quality_of_service_returns_default() {
        assert_eq!(shim_queue_quality_of_service(0, 0), -1);
    }

    #[test]
    fn queue_set_quality_of_service_is_noop() {
        shim_queue_set_quality_of_service(0, 0, 0x21);
        // no panic = pass
    }
}
