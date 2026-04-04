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

// _Block_object_assign / _Block_object_dispose field-type flags (low byte).
const BLOCK_FIELD_IS_OBJECT: i32 = 3; // captured ObjC object
const BLOCK_FIELD_IS_BLOCK: i32 = 7; // captured nested block
const BLOCK_FIELD_IS_BYREF: i32 = 8; // captured __block variable

/// 8-byte sentinel for `_NSConcreteStackBlock`.
static NSCONCRETESTACK_SENTINEL: [u8; 8] = [0x53, 0x54, 0x4B, 0x42, 0x4C, 0x4B, 0x00, 0x00];

/// 8-byte sentinel for `_NSConcreteGlobalBlock`.
static NSCONCRETEGLOBAL_SENTINEL: [u8; 8] = [0x47, 0x4C, 0x42, 0x42, 0x4C, 0x4B, 0x00, 0x00];

pub struct BlocksRuntimeStubs;

impl BlocksRuntimeStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("__Block_copy", shim_block_copy as *const () as u64, "Block_copy — copy a block to the heap"),
            ("__Block_release", shim_block_release as *const () as u64, "Block_release — release a heap block"),
            ("__Block_object_assign", shim_block_object_assign as *const () as u64, "Block_object_assign — captured object assign/copy"),
            ("__Block_object_dispose", shim_block_object_dispose as *const () as u64, "Block_object_dispose — captured object dispose/release"),
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

/// Read the descriptor pointer at offset 24.
unsafe fn read_descriptor(block: u64) -> u64 {
    std::ptr::read_unaligned((block as *const u8).add(24) as *const u64)
}

/// `_Block_copy(block) -> block`
///
/// If global, returns as-is.  Otherwise allocates a heap copy with
/// `BLOCK_NEEDS_FREE` set and initial refcount = 1.  If the block has
/// `BLOCK_HAS_COPY_DISPOSE`, invokes the copy helper from the descriptor.
pub extern "C" fn shim_block_copy(block: u64) -> u64 {
    if block == 0 {
        return 0;
    }
    unsafe {
        let flags = read_flags(block);
        if flags & BLOCK_IS_GLOBAL != 0 {
            return block;
        }
        // Already on heap — bump refcount.
        if flags & BLOCK_NEEDS_FREE != 0 {
            let rc = flags & 0xFFFF;
            write_flags(block, (flags & !0xFFFF) | (rc + 1));
            return block;
        }
        // Determine size from descriptor (fallback to BLOCK_SIZE).
        let desc = read_descriptor(block);
        let size = if desc != 0 {
            let s = std::ptr::read_unaligned((desc as *const u8).add(8) as *const u64) as usize;
            if s >= BLOCK_SIZE { s } else { BLOCK_SIZE }
        } else {
            BLOCK_SIZE
        };
        // Allocate heap copy.
        let heap = libc::malloc(size);
        if heap.is_null() {
            log::error!("_Block_copy: malloc failed");
            return 0;
        }
        std::ptr::copy_nonoverlapping(block as *const u8, heap as *mut u8, size);
        // Set BLOCK_NEEDS_FREE and initial refcount = 1 (bits 0-15).
        let new_flags = (flags & !0xFFFF) | BLOCK_NEEDS_FREE | 1;
        write_flags(heap as u64, new_flags);
        // Call copy helper if BLOCK_HAS_COPY_DISPOSE.
        if flags & BLOCK_HAS_COPY_DISPOSE != 0 && desc != 0 {
            let copy_fn = std::ptr::read_unaligned((desc as *const u8).add(16) as *const u64);
            if copy_fn != 0 {
                let copy: extern "C" fn(u64, u64) = std::mem::transmute(copy_fn);
                copy(heap as u64, block);
            }
        }
        heap as u64
    }
}

/// `_Block_release(block)`
///
/// If global or null, no-op.  If heap (`BLOCK_NEEDS_FREE`), decrements
/// refcount.  At zero, invokes the dispose helper (if present) then frees.
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
                // Call dispose helper before freeing.
                if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
                    let desc = read_descriptor(block);
                    if desc != 0 {
                        let dispose_fn =
                            std::ptr::read_unaligned((desc as *const u8).add(24) as *const u64);
                        if dispose_fn != 0 {
                            let dispose: extern "C" fn(u64) = std::mem::transmute(dispose_fn);
                            dispose(block);
                        }
                    }
                }
                libc::free(block as *mut libc::c_void);
            } else {
                write_flags(block, (flags & !0xFFFF) | (refcount - 1));
            }
        }
    }
}

/// `_Block_object_assign(dest_addr, object, flags)`
///
/// Called by compiler-generated copy helpers to retain/copy captured values.
/// - `BLOCK_FIELD_IS_OBJECT` → store the pointer (no ObjC retain without runtime).
/// - `BLOCK_FIELD_IS_BLOCK`  → `Block_copy` the nested block, store result.
/// - `BLOCK_FIELD_IS_BYREF`  → copy byref struct to heap, update forwarding pointers.
pub extern "C" fn shim_block_object_assign(dest_addr: u64, object: u64, flags: i32) {
    if dest_addr == 0 || object == 0 {
        return;
    }
    let kind = flags & 0xFF;
    match kind {
        BLOCK_FIELD_IS_BLOCK => {
            let copied = shim_block_copy(object);
            unsafe { *(dest_addr as *mut u64) = copied; }
        }
        BLOCK_FIELD_IS_BYREF => {
            // Copy byref struct to heap and update forwarding pointers.
            // Byref layout: [isa:8][forwarding:8][flags:4][size:4][...].
            let forwarding = unsafe {
                std::ptr::read_unaligned((object as *const u8).add(8) as *const u64)
            };
            // If forwarding already points elsewhere, it's already on the heap
            if forwarding != object && forwarding != 0 {
                unsafe { *(dest_addr as *mut u64) = forwarding; }
                return;
            }
            // Read size from offset 20
            let size = unsafe {
                std::ptr::read_unaligned((object as *const u8).add(20) as *const u32)
            } as usize;
            let alloc_size = if size >= BLOCK_SIZE { size } else { BLOCK_SIZE };
            let heap = unsafe { libc::malloc(alloc_size) };
            if heap.is_null() {
                unsafe { *(dest_addr as *mut u64) = object; }
                return;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(object as *const u8, heap as *mut u8, alloc_size);
                // Heap copy forwarding → itself
                std::ptr::write_unaligned((heap as *mut u8).add(8) as *mut u64, heap as u64);
                // Stack copy forwarding → heap
                std::ptr::write_unaligned((object as *mut u8).add(8) as *mut u64, heap as u64);
                // Store heap pointer at destination
                *(dest_addr as *mut u64) = heap as u64;
            }
        }
        // IS_OBJECT or unknown — store pointer as-is.
        _ => {
            unsafe { *(dest_addr as *mut u64) = object; }
        }
    }
}

/// `_Block_object_dispose(object, flags)`
///
/// Called by compiler-generated dispose helpers to release captured values.
/// - `BLOCK_FIELD_IS_BLOCK` → `Block_release` the nested block.
/// - Others                 → no-op (no ObjC runtime available here).
pub extern "C" fn shim_block_object_dispose(object: u64, flags: i32) {
    if object == 0 {
        return;
    }
    let kind = flags & 0xFF;
    if kind == BLOCK_FIELD_IS_BLOCK {
        shim_block_release(object);
    }
    // IS_OBJECT: would need ObjC release — libsystem-shims doesn't have it.
    // IS_BYREF: no-op (simplified).
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
        // Null dest or null object should be no-ops.
        shim_block_object_assign(0, 0x2000, 0);
        shim_block_object_assign(0, 0, BLOCK_HAS_COPY_DISPOSE);
        // Valid dest with valid object (unknown flag kind = 0).
        let mut dest: u64 = 0;
        shim_block_object_assign(&mut dest as *mut u64 as u64, 0x2000, 0);
        assert_eq!(dest, 0x2000);
    }

    #[test]
    fn test_block_object_assign_stores_pointer() {
        let mut dest: u64 = 0;
        let obj: u64 = 0xDEAD_BEEF;
        shim_block_object_assign(&mut dest as *mut u64 as u64, obj, BLOCK_FIELD_IS_OBJECT);
        assert_eq!(dest, obj, "IS_OBJECT should store pointer");
    }

    #[test]
    fn test_block_object_assign_byref_stores_pointer() {
        let mut dest: u64 = 0;
        let obj: u64 = 0xCAFE_BABE;
        shim_block_object_assign(&mut dest as *mut u64 as u64, obj, BLOCK_FIELD_IS_BYREF);
        assert_eq!(dest, obj, "IS_BYREF should store pointer");
    }

    #[test]
    fn test_block_object_assign_block_copies() {
        // Create a global block (copy returns same pointer).
        let mut blk = [0u8; BLOCK_SIZE];
        make_block(&mut blk, BLOCK_IS_GLOBAL, 0xF00D);
        let mut dest: u64 = 0;
        shim_block_object_assign(
            &mut dest as *mut u64 as u64,
            blk.as_ptr() as u64,
            BLOCK_FIELD_IS_BLOCK,
        );
        assert_eq!(dest, blk.as_ptr() as u64, "IS_BLOCK on global should store same ptr");
    }

    #[test]
    fn test_block_object_assign_null_safe() {
        // null dest or null object should not crash.
        shim_block_object_assign(0, 0xBEEF, BLOCK_FIELD_IS_OBJECT);
        shim_block_object_assign(0xBEEF, 0, BLOCK_FIELD_IS_OBJECT);
    }

    #[test]
    fn test_block_object_dispose_null_safe() {
        shim_block_object_dispose(0, BLOCK_FIELD_IS_OBJECT);
        shim_block_object_dispose(0, BLOCK_FIELD_IS_BLOCK);
    }

    #[test]
    fn test_block_copy_with_descriptor_size() {
        // Build a block with a descriptor that specifies size = 48.
        let mut desc = [0u8; 32];
        unsafe {
            // desc+0: reserved = 0
            // desc+8: size = 48
            std::ptr::write_unaligned(desc.as_mut_ptr().add(8) as *mut u64, 48u64);
        }
        let mut buf = [0u8; 48];
        buf.fill(0);
        unsafe {
            // flags at offset 8 — no special flags (stack block).
            std::ptr::write_unaligned(buf.as_mut_ptr().add(8) as *mut i32, 0);
            // invoke at offset 16.
            std::ptr::write_unaligned(buf.as_mut_ptr().add(16) as *mut u64, 0xCAFE);
            // descriptor at offset 24.
            std::ptr::write_unaligned(
                buf.as_mut_ptr().add(24) as *mut u64,
                desc.as_ptr() as u64,
            );
        }
        let heap = shim_block_copy(buf.as_ptr() as u64);
        assert_ne!(heap, 0);
        assert_ne!(heap, buf.as_ptr() as u64);
        shim_block_release(heap);
    }

    #[test]
    fn test_heap_block_copy_bumps_refcount() {
        let mut buf = [0u8; BLOCK_SIZE];
        make_block(&mut buf, 0, 0xBEEF);
        let heap = shim_block_copy(buf.as_ptr() as u64);
        assert_ne!(heap, 0);

        // Copy the heap block again — should bump refcount.
        let heap2 = shim_block_copy(heap);
        assert_eq!(heap2, heap, "heap copy should return same pointer");
        unsafe {
            let flags = read_flags(heap);
            assert_eq!(flags & 0xFFFF, 2, "refcount should be 2 after second copy");
        }

        // Two releases to free.
        shim_block_release(heap);
        shim_block_release(heap);
    }
}
