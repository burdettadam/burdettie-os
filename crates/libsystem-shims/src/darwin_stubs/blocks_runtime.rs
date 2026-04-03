//! Blocks runtime API shims (`_Block_copy`, `_Block_release`, etc.).
//!
//! On macOS these resolve via dlsym to libclosure.  On Linux these need
//! to be provided when libBlocksRuntime isn't available.
//!
//! Block layout (Apple Blocks ABI):
//!   [0..8):   isa         — class pointer (_NSConcreteStackBlock / _NSConcreteGlobalBlock)
//!   [8..12):  flags       — bit 24 = NEEDS_FREE, bit 25 = HAS_COPY_DISPOSE,
//!                           bit 28 = IS_GLOBAL, bits 0-15 = refcount (heap)
//!   [12..16): reserved
//!   [16..24): invoke      — function pointer
//!   [24..32): descriptor  — pointer to Block_descriptor

use crate::table::ShimTable;

const BLOCK_SIZE: usize = 32;

const BLOCK_HAS_COPY_DISPOSE: i32 = 1 << 25;
const BLOCK_IS_GLOBAL: i32 = 1 << 28;
const BLOCK_NEEDS_FREE: i32 = 1 << 24;

/// 8-byte sentinel for `_NSConcreteStackBlock`.
static NSCONCRETESTACK_SENTINEL: [u8; 8] = [0x53, 0x54, 0x4B, 0x42, 0x4C, 0x4B, 0x00, 0x00];

/// 8-byte sentinel for `_NSConcreteGlobalBlock`.
static NSCONCRETEGLOBAL_SENTINEL: [u8; 8] = [0x47, 0x4C, 0x42, 0x42, 0x4C, 0x4B, 0x00, 0x00];

pub struct BlocksRuntimeStubs;

impl BlocksRuntimeStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("__Block_copy", shim_block_copy as u64, "Block_copy — copy a block to the heap"),
            ("__Block_release", shim_block_release as u64, "Block_release — release a heap block"),
            ("__Block_object_assign", shim_block_object_assign as u64, "Block_object_assign — captured object assign (no-op)"),
            ("__Block_object_dispose", shim_block_object_dispose as u64, "Block_object_dispose — captured object dispose (no-op)"),
            ("__NSConcreteStackBlock", NSCONCRETESTACK_SENTINEL.as_ptr() as u64, "NSConcreteStackBlock data symbol"),
            ("__NSConcreteGlobalBlock", NSCONCRETEGLOBAL_SENTINEL.as_ptr() as u64, "NSConcreteGlobalBlock data symbol"),
        );
        log::debug!("BlocksRuntimeStubs: registered 6 stubs");
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read the `flags` field (i32) at offset 8 from a block pointer.
unsafe fn read_flags(block: u64) -> i32 {
    let ptr = block as *const u8;
    std::ptr::read_unaligned(ptr.add(8) as *const i32)
}

/// Write the `flags` field (i32) at offset 8 of a block pointer.
unsafe fn write_flags(block: u64, flags: i32) {
    let ptr = block as *mut u8;
    std::ptr::write_unaligned(ptr.add(8) as *mut i32, flags);
}

// ---------------------------------------------------------------------------
// Shim implementations
// ---------------------------------------------------------------------------

/// `_Block_copy(block) -> block`
///
/// If global, returns as-is.  Otherwise allocates a heap copy with
/// `BLOCK_NEEDS_FREE` set and initial refcount = 1.
pub extern "C" fn shim_block_copy(block: u64) -> u64 {
    if block == 0 {
        return 0;
    }
    unsafe {
        let flags = read_flags(block);
        if flags & BLOCK_IS_GLOBAL != 0 {
            return block;
        }
        // Allocate heap copy.
        let heap = libc::malloc(BLOCK_SIZE);
        if heap.is_null() {
            log::error!("_Block_copy: malloc failed");
            return 0;
        }
        std::ptr::copy_nonoverlapping(block as *const u8, heap as *mut u8, BLOCK_SIZE);
        // Set BLOCK_NEEDS_FREE and initial refcount = 1 (bits 0-15).
        let new_flags = (flags & !0xFFFF) | BLOCK_NEEDS_FREE | 1;
        write_flags(heap as u64, new_flags);
        heap as u64
    }
}

/// `_Block_release(block)`
///
/// If global or null, no-op.  If heap (`BLOCK_NEEDS_FREE`), decrements
/// refcount and frees when it reaches zero.
pub extern "C" fn shim_block_release(block: u64) {
    if block == 0 {
        return;
    }
    unsafe {
        let flags = read_flags(block);
        if flags & BLOCK_IS_GLOBAL != 0 {
            return;
        }
        if flags & BLOCK_NEEDS_FREE != 0 {
            let refcount = flags & 0xFFFF;
            if refcount <= 1 {
                libc::free(block as *mut libc::c_void);
            } else {
                write_flags(block, (flags & !0xFFFF) | (refcount - 1));
            }
        }
    }
}

/// `_Block_object_assign(dest_addr, object, flags)` — no-op stub.
///
/// Called by compiler-generated copy helpers to assign captured objects
/// with ARC semantics.  We skip full ARC for block captures.
pub extern "C" fn shim_block_object_assign(dest_addr: u64, object: u64, flags: i32) {
    log::trace!(
        "_Block_object_assign: dest={:#x} obj={:#x} flags={:#x} (no-op)",
        dest_addr,
        object,
        flags,
    );
}

/// `_Block_object_dispose(object, flags)` — no-op stub.
///
/// Called by compiler-generated dispose helpers.
pub extern "C" fn shim_block_object_dispose(object: u64, flags: i32) {
    log::trace!(
        "_Block_object_dispose: obj={:#x} flags={:#x} (no-op)",
        object,
        flags,
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a fake block on the stack as a `[u8; 32]` array with
    /// the given flags and invoke pointer, returning a mutable reference
    /// to the backing storage.
    fn make_block(buf: &mut [u8; BLOCK_SIZE], flags: i32, invoke: u64) {
        buf.fill(0);
        unsafe {
            // isa at offset 0 — leave zero
            // flags at offset 8
            std::ptr::write_unaligned(buf.as_mut_ptr().add(8) as *mut i32, flags);
            // invoke at offset 16
            std::ptr::write_unaligned(buf.as_mut_ptr().add(16) as *mut u64, invoke);
        }
    }

    #[test]
    fn test_block_copy_null() {
        assert_eq!(shim_block_copy(0), 0);
    }

    #[test]
    fn test_block_release_null() {
        shim_block_release(0);
    }

    #[test]
    fn test_global_block_copy_identity() {
        let mut buf = [0u8; BLOCK_SIZE];
        make_block(&mut buf, BLOCK_IS_GLOBAL, 0xDEAD);
        let ptr = buf.as_ptr() as u64;
        let copied = shim_block_copy(ptr);
        assert_eq!(copied, ptr, "global block copy should return same pointer");
    }

    #[test]
    fn test_stack_block_copy_allocates() {
        let mut buf = [0u8; BLOCK_SIZE];
        make_block(&mut buf, 0, 0xCAFE);
        let src = buf.as_ptr() as u64;
        let heap = shim_block_copy(src);
        assert_ne!(heap, 0, "heap copy should be non-null");
        assert_ne!(heap, src, "heap copy should differ from original");

        // Verify BLOCK_NEEDS_FREE is set and refcount == 1.
        unsafe {
            let flags = read_flags(heap);
            assert_ne!(flags & BLOCK_NEEDS_FREE, 0, "NEEDS_FREE should be set");
            assert_eq!(flags & 0xFFFF, 1, "refcount should be 1");
        }

        // Release to free.
        shim_block_release(heap);
    }

    #[test]
    fn test_block_copy_preserves_invoke() {
        let invoke_addr: u64 = 0x0042_CAFE_BABE_0000;
        let mut buf = [0u8; BLOCK_SIZE];
        make_block(&mut buf, 0, invoke_addr);
        let heap = shim_block_copy(buf.as_ptr() as u64);
        assert_ne!(heap, 0);

        unsafe {
            let invoke = std::ptr::read_unaligned((heap as *const u8).add(16) as *const u64);
            assert_eq!(invoke, invoke_addr, "invoke pointer should be preserved");
        }

        shim_block_release(heap);
    }

    #[test]
    fn test_block_object_assign_no_crash() {
        shim_block_object_assign(0x1000, 0x2000, 0);
        shim_block_object_assign(0, 0, BLOCK_HAS_COPY_DISPOSE);
    }
}
