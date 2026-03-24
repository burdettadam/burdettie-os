//! Blocks runtime support: _Block_copy, _Block_release, and class symbols.
//!
//! Implements the Objective-C blocks ABI (Apple's "Block Implementation
//! Specification"). Supports global blocks (no captures), stack blocks
//! (Block_copy to heap), and malloc blocks (refcounted on heap).
//!
//! Layout of a block literal (Block_literal_1):
//!
//! ```text
//! +0   void *isa            // class pointer
//! +8   int32 flags          // see BLOCK_* constants
//! +12  int32 reserved       // always 0
//! +16  void (*invoke)(...)  // function pointer
//! +24  Block_descriptor *   // pointer to descriptor struct
//! +32  ...                  // captured variables
//! ```
//!
//! Block_descriptor_1:
//! ```text
//! +0   unsigned long reserved   // always 0
//! +8   unsigned long size       // sizeof(Block_literal)
//! +16  void (*copy)(...)        // present if BLOCK_HAS_COPY_DISPOSE
//! +24  void (*dispose)(...)     // present if BLOCK_HAS_COPY_DISPOSE
//! ```

use std::sync::OnceLock;

use crate::refcount::REFCOUNTS;

// ── Block flags ────────────────────────────────────────────────────

const BLOCK_NEEDS_FREE: u32 = 1 << 24;
const BLOCK_HAS_COPY_DISPOSE: u32 = 1 << 25;
const BLOCK_IS_GLOBAL: u32 = 1 << 28;

// ── Block layout offsets ───────────────────────────────────────────

const FLAGS_OFF: usize = 8;
const DESC_OFF: usize = 24;
const MIN_BLOCK_SIZE: usize = 32;

// Descriptor offsets
const DESC_SIZE_OFF: usize = 8;
const DESC_COPY_OFF: usize = 16;
const DESC_DISPOSE_OFF: usize = 24;

// ── Class data symbols ─────────────────────────────────────────────
//
// Clang references `__NSConcreteGlobalBlock` and `__NSConcreteStackBlock`
// as class pointers written into the `isa` field of block literals.
// We provide them as stable addresses.

/// 8-byte aligned storage for a fake class pointer.
/// We only need a stable, non-zero address — no actual class struct.
#[repr(align(8))]
struct ClassSlot([u8; 8]);

static GLOBAL_SLOT: ClassSlot = ClassSlot([0x47, 0x42, 0x4C, 0x4B, 0, 0, 0, 0]); // "GBLK"
static STACK_SLOT: ClassSlot = ClassSlot([0x53, 0x42, 0x4C, 0x4B, 0, 0, 0, 0]); // "SBLK"
static MALLOC_SLOT: ClassSlot = ClassSlot([0x4D, 0x42, 0x4C, 0x4B, 0, 0, 0, 0]); // "MBLK"

static GLOBAL_BLOCK_CLASS: OnceLock<u64> = OnceLock::new();
static STACK_BLOCK_CLASS: OnceLock<u64> = OnceLock::new();
static MALLOC_BLOCK_CLASS: OnceLock<u64> = OnceLock::new();

/// Address of `_NSConcreteGlobalBlock` class reference.
pub fn global_block_class_addr() -> u64 {
    *GLOBAL_BLOCK_CLASS.get_or_init(|| GLOBAL_SLOT.0.as_ptr() as u64)
}

/// Address of `_NSConcreteStackBlock` class reference.
pub fn stack_block_class_addr() -> u64 {
    *STACK_BLOCK_CLASS.get_or_init(|| STACK_SLOT.0.as_ptr() as u64)
}

/// Address of `_NSConcreteMallocBlock` class reference.
fn malloc_block_class_addr() -> u64 {
    *MALLOC_BLOCK_CLASS.get_or_init(|| MALLOC_SLOT.0.as_ptr() as u64)
}

// ── _Block_copy ────────────────────────────────────────────────────

/// `void *_Block_copy(const void *block)`
///
/// - Global block → return as-is (statically allocated).
/// - Malloc block → bump refcount, return same pointer.
/// - Stack block → allocate heap copy, set isa = MallocBlock.
pub extern "C" fn shim_block_copy(block: u64) -> u64 {
    if block == 0 {
        return 0;
    }

    let isa = unsafe { *(block as *const u64) };
    let flags = unsafe { *((block as usize + FLAGS_OFF) as *const u32) };

    // Global block — return as-is
    if flags & BLOCK_IS_GLOBAL != 0 || isa == global_block_class_addr() {
        return block;
    }

    // Already on heap (malloc block) — bump refcount
    if flags & BLOCK_NEEDS_FREE != 0 {
        REFCOUNTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(block);
        return block;
    }

    // Stack block — copy to heap
    let desc = unsafe { *((block as usize + DESC_OFF) as *const u64) };
    let size = if desc != 0 {
        let s = unsafe { *((desc as usize + DESC_SIZE_OFF) as *const u64) } as usize;
        if s >= MIN_BLOCK_SIZE {
            s
        } else {
            MIN_BLOCK_SIZE
        }
    } else {
        MIN_BLOCK_SIZE
    };

    let heap = unsafe { libc::malloc(size) } as u64;
    if heap == 0 {
        return 0;
    }

    // Bitwise copy
    unsafe {
        std::ptr::copy_nonoverlapping(block as *const u8, heap as *mut u8, size);
    }

    // Set isa = _NSConcreteMallocBlock
    unsafe {
        *(heap as *mut u64) = malloc_block_class_addr();
    }

    // Set BLOCK_NEEDS_FREE in flags
    unsafe {
        let f = (heap as usize + FLAGS_OFF) as *mut u32;
        *f |= BLOCK_NEEDS_FREE;
    }

    // Initial refcount = 1 (the new heap copy)
    REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(heap);

    // Call copy helper if BLOCK_HAS_COPY_DISPOSE
    if flags & BLOCK_HAS_COPY_DISPOSE != 0 && desc != 0 {
        let copy_fn = unsafe { *((desc as usize + DESC_COPY_OFF) as *const u64) };
        if copy_fn != 0 {
            let copy: extern "C" fn(u64, u64) = unsafe { std::mem::transmute(copy_fn) };
            copy(heap, block);
        }
    }

    log::debug!(
        "_Block_copy: copied stack block {:#x} → heap {:#x} ({} bytes)",
        block,
        heap,
        size
    );
    heap
}

// ── _Block_release ─────────────────────────────────────────────────

/// `void _Block_release(const void *block)`
///
/// - Global/stack block → no-op.
/// - Malloc block → decrement refcount, free at 0.
pub extern "C" fn shim_block_release(block: u64) {
    if block == 0 {
        return;
    }

    let flags = unsafe { *((block as usize + FLAGS_OFF) as *const u32) };

    // Only malloc'd blocks are refcounted
    if flags & BLOCK_NEEDS_FREE == 0 {
        return;
    }

    let remaining = REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .release(block);

    if remaining == 0 {
        // Call dispose helper if present
        if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
            let desc = unsafe { *((block as usize + DESC_OFF) as *const u64) };
            if desc != 0 {
                let dispose_fn = unsafe { *((desc as usize + DESC_DISPOSE_OFF) as *const u64) };
                if dispose_fn != 0 {
                    let dispose: extern "C" fn(u64) = unsafe { std::mem::transmute(dispose_fn) };
                    dispose(block);
                }
            }
        }
        log::debug!("_Block_release: freed malloc block {:#x}", block);
        unsafe { libc::free(block as *mut libc::c_void) };
    }
}

// ── objc_retainBlock ───────────────────────────────────────────────

/// `id objc_retainBlock(id block)`
///
/// ARC calls this to retain a block — it's equivalent to Block_copy.
pub extern "C" fn shim_objc_retain_block(block: u64) -> u64 {
    shim_block_copy(block)
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_addresses_are_stable_and_distinct() {
        let g = global_block_class_addr();
        let s = stack_block_class_addr();
        let m = malloc_block_class_addr();
        assert_ne!(g, 0);
        assert_ne!(s, 0);
        assert_ne!(m, 0);
        assert_ne!(g, s);
        assert_ne!(g, m);
        assert_ne!(s, m);
        // Stable across calls
        assert_eq!(g, global_block_class_addr());
        assert_eq!(s, stack_block_class_addr());
        assert_eq!(m, malloc_block_class_addr());
    }

    #[test]
    fn block_copy_null_returns_null() {
        assert_eq!(shim_block_copy(0), 0);
    }

    #[test]
    fn block_release_null_is_noop() {
        shim_block_release(0); // should not panic
    }

    #[test]
    fn global_block_copy_returns_same() {
        // Simulate a global block: isa = global, flags = BLOCK_IS_GLOBAL
        #[repr(C)]
        struct FakeGlobal {
            isa: u64,
            flags: u32,
            reserved: u32,
            invoke: u64,
            descriptor: u64,
        }
        let blk = FakeGlobal {
            isa: global_block_class_addr(),
            flags: BLOCK_IS_GLOBAL,
            reserved: 0,
            invoke: 0xDEAD,
            descriptor: 0,
        };
        let addr = &blk as *const _ as u64;
        assert_eq!(shim_block_copy(addr), addr);
    }

    #[test]
    fn stack_block_copy_creates_heap_copy() {
        // Simulate a stack block with descriptor
        #[repr(C)]
        struct FakeDesc {
            reserved: u64,
            size: u64,
        }
        #[repr(C)]
        struct FakeStack {
            isa: u64,
            flags: u32,
            reserved: u32,
            invoke: u64,
            descriptor: u64,
        }
        let desc = FakeDesc {
            reserved: 0,
            size: std::mem::size_of::<FakeStack>() as u64,
        };
        let blk = FakeStack {
            isa: stack_block_class_addr(),
            flags: 0, // no BLOCK_IS_GLOBAL, no BLOCK_NEEDS_FREE → stack block
            reserved: 0,
            invoke: 0xCAFE,
            descriptor: &desc as *const _ as u64,
        };
        let stack_addr = &blk as *const _ as u64;
        let heap_addr = shim_block_copy(stack_addr);

        // Must be a new allocation
        assert_ne!(heap_addr, 0);
        assert_ne!(heap_addr, stack_addr);

        // isa should be malloc block class
        let new_isa = unsafe { *(heap_addr as *const u64) };
        assert_eq!(new_isa, malloc_block_class_addr());

        // invoke pointer should be preserved
        let new_invoke = unsafe { *((heap_addr as usize + 16) as *const u64) };
        assert_eq!(new_invoke, 0xCAFE);

        // flags should have BLOCK_NEEDS_FREE
        let new_flags = unsafe { *((heap_addr as usize + FLAGS_OFF) as *const u32) };
        assert!(new_flags & BLOCK_NEEDS_FREE != 0);

        // Release the copy
        shim_block_release(heap_addr);
    }

    #[test]
    fn block_copy_retain_release_cycle() {
        // Create a heap block via copy
        #[repr(C)]
        struct FakeDesc {
            reserved: u64,
            size: u64,
        }
        #[repr(C)]
        struct FakeStack {
            isa: u64,
            flags: u32,
            reserved: u32,
            invoke: u64,
            descriptor: u64,
        }

        let desc = FakeDesc {
            reserved: 0,
            size: 32,
        };
        let blk = FakeStack {
            isa: stack_block_class_addr(),
            flags: 0,
            reserved: 0,
            invoke: 0xBEEF,
            descriptor: &desc as *const _ as u64,
        };
        let heap = shim_block_copy(&blk as *const _ as u64);
        assert_ne!(heap, 0);

        // Copy again → bumps refcount, same pointer
        let heap2 = shim_block_copy(heap);
        assert_eq!(heap2, heap);

        // Release twice → should free on second
        shim_block_release(heap);
        shim_block_release(heap);
        // After double release the block is freed — no crash means success
    }
}
