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

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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
        REFCOUNTS.retain(block);
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

    let heap = crate::pal_alloc().malloc(size) as u64;
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
    REFCOUNTS.retain(heap);

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

    let remaining = REFCOUNTS.release(block);

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
        crate::pal_alloc().free(block as *mut u8);
    }
}

// ── objc_retainBlock ───────────────────────────────────────────────

/// `id objc_retainBlock(id block)`
///
/// ARC calls this to retain a block — it's equivalent to Block_copy.
pub extern "C" fn shim_objc_retain_block(block: u64) -> u64 {
    shim_block_copy(block)
}

// ── imp_implementationWithBlock / imp_getBlock / imp_removeBlock ───

/// Global table mapping IMP addresses back to their source block.
/// Used by imp_getBlock and imp_removeBlock.
static IMP_BLOCK_MAP: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

fn imp_block_map_insert(imp: u64, block: u64) {
    let mut guard = IMP_BLOCK_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(imp, block);
}

fn imp_block_map_get(imp: u64) -> Option<u64> {
    let guard = IMP_BLOCK_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().and_then(|m| m.get(&imp).copied())
}

fn imp_block_map_remove(imp: u64) -> Option<u64> {
    let mut guard = IMP_BLOCK_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().and_then(|m| m.remove(&imp))
}

/// `IMP imp_implementationWithBlock(id block)`
///
/// Creates an IMP from a block. We copy the block (to retain it),
/// extract its invoke function pointer, and store the mapping so that
/// imp_getBlock and imp_removeBlock can operate on it.
///
/// Returns the block's invoke function pointer as the IMP.
/// If the invoke pointer collides with a prior mapping, the new block
/// wins (matching Apple behavior for same-signature blocks).
pub extern "C" fn shim_imp_implementation_with_block(block: u64) -> u64 {
    if block == 0 {
        return 0;
    }
    let copied = shim_block_copy(block);
    if copied == 0 {
        return 0;
    }
    // invoke is at offset +16 in Block_literal
    let invoke = unsafe { *((copied as usize + 16) as *const u64) };
    if invoke == 0 {
        return 0;
    }
    imp_block_map_insert(invoke, copied);
    invoke
}

/// `id imp_getBlock(IMP imp)`
///
/// Returns the block associated with an IMP created by
/// imp_implementationWithBlock, or NULL if no block is associated.
pub extern "C" fn shim_imp_get_block(imp: u64) -> u64 {
    if imp == 0 {
        return 0;
    }
    imp_block_map_get(imp).unwrap_or(0)
}

/// `BOOL imp_removeBlock(IMP imp)`
///
/// Removes the block association for the given IMP and releases the block.
/// Returns 1 (YES) if a block was removed, 0 (NO) otherwise.
pub extern "C" fn shim_imp_remove_block(imp: u64) -> u8 {
    if imp == 0 {
        return 0;
    }
    if let Some(block) = imp_block_map_remove(imp) {
        shim_block_release(block);
        1
    } else {
        0
    }
}

// ── _Block_object_assign / _Block_object_dispose ──────────────────

// Flags for _Block_object_assign / _Block_object_dispose.
// From Block_private.h:
const BLOCK_FIELD_IS_OBJECT: u32 = 3; // captured ObjC object
const BLOCK_FIELD_IS_BLOCK: u32 = 7; // captured __block block
const BLOCK_FIELD_IS_BYREF: u32 = 8; // captured __block variable
const BLOCK_BYREF_CALLER: u32 = 128; // called from byref copy/dispose helper

// Byref struct layout offsets.
const BYREF_FORWARDING_OFF: usize = 8;
const BYREF_FLAGS_OFF: usize = 16;
const BYREF_SIZE_OFF: usize = 20;
const BYREF_HAS_COPY_DISPOSE: u32 = 1 << 25;
const BYREF_NEEDS_FREE: u32 = 1 << 24;
const BYREF_MIN_SIZE: usize = 32;

/// Copy a `__block` byref struct from stack to heap, updating both
/// forwarding pointers.  Returns the heap address.
fn copy_byref_to_heap(byref: u64) -> u64 {
    // Read the existing forwarding pointer — if it already points to heap, bump refcount
    let forwarding = unsafe { *((byref as usize + BYREF_FORWARDING_OFF) as *const u64) };
    if forwarding != byref && forwarding != 0 {
        // Already copied to heap — return existing heap copy
        REFCOUNTS.retain(forwarding);
        return forwarding;
    }

    // Read size from the byref struct
    let size = unsafe { *((byref as usize + BYREF_SIZE_OFF) as *const u32) } as usize;
    let alloc_size = if size >= BYREF_MIN_SIZE {
        size
    } else {
        BYREF_MIN_SIZE
    };

    let heap = crate::pal_alloc().malloc(alloc_size) as u64;
    if heap == 0 {
        return byref;
    }

    // Bitwise copy
    unsafe {
        std::ptr::copy_nonoverlapping(byref as *const u8, heap as *mut u8, alloc_size);
    }

    // Set heap copy's forwarding to itself
    unsafe {
        *((heap as usize + BYREF_FORWARDING_OFF) as *mut u64) = heap;
    }

    // Update stack copy's forwarding to point to heap
    unsafe {
        *((byref as usize + BYREF_FORWARDING_OFF) as *mut u64) = heap;
    }

    // Set BYREF_NEEDS_FREE on heap copy
    unsafe {
        let f = (heap as usize + BYREF_FLAGS_OFF) as *mut u32;
        *f |= BYREF_NEEDS_FREE;
    }

    // Initial refcount = 1
    REFCOUNTS.retain(heap);

    // Call byref copy helper if HAS_COPY_DISPOSE
    let flags = unsafe { *((byref as usize + BYREF_FLAGS_OFF) as *const u32) };
    if flags & BYREF_HAS_COPY_DISPOSE != 0 {
        let copy_fn = unsafe { *((byref as usize + 24) as *const u64) };
        if copy_fn != 0 {
            let copy: extern "C" fn(u64, u64) = unsafe { std::mem::transmute(copy_fn) };
            copy(heap, byref);
        }
    }

    log::debug!(
        "_Block_object_assign: copied byref {:#x} → heap {:#x} ({} bytes)",
        byref,
        heap,
        alloc_size
    );
    heap
}

/// `void _Block_object_assign(void *destAddr, const void *object, const int flags)`
///
/// Called inside block copy helpers to retain/copy captured objects.
/// - BLOCK_FIELD_IS_OBJECT → retain the object
/// - BLOCK_FIELD_IS_BLOCK  → Block_copy the nested block
/// - BLOCK_FIELD_IS_BYREF  → copy the __block byref struct to heap, update forwarding
pub extern "C" fn shim_block_object_assign(dest_addr: u64, object: u64, flags: u32) {
    if dest_addr == 0 || object == 0 {
        return;
    }
    let kind = flags & 0xFF; // low byte is the field type
    match kind {
        BLOCK_FIELD_IS_OBJECT => {
            // Retain the ObjC object and store pointer
            crate::refcount::REFCOUNTS.retain(object);
            unsafe {
                *(dest_addr as *mut u64) = object;
            }
        }
        BLOCK_FIELD_IS_BLOCK => {
            // Block_copy the nested block
            let copied = shim_block_copy(object);
            unsafe {
                *(dest_addr as *mut u64) = copied;
            }
        }
        _ if kind == BLOCK_FIELD_IS_BYREF
            || kind == (BLOCK_FIELD_IS_BYREF | BLOCK_BYREF_CALLER) =>
        {
            // __block byref: copy to heap and update forwarding pointers
            let heap = copy_byref_to_heap(object);
            unsafe {
                *(dest_addr as *mut u64) = heap;
            }
        }
        _ => {
            // Unknown flag combination — store pointer as-is
            log::debug!(
                "_Block_object_assign: unknown flags {:#x}, storing pointer",
                flags
            );
            unsafe {
                *(dest_addr as *mut u64) = object;
            }
        }
    }
}

/// `void _Block_object_dispose(const void *object, const int flags)`
///
/// Called inside block dispose helpers to release captured objects.
/// - BLOCK_FIELD_IS_OBJECT → release the object
/// - BLOCK_FIELD_IS_BLOCK  → Block_release the nested block
/// - BLOCK_FIELD_IS_BYREF  → release the __block byref struct (simplified: no-op)
pub extern "C" fn shim_block_object_dispose(object: u64, flags: u32) {
    if object == 0 {
        return;
    }
    let kind = flags & 0xFF;
    match kind {
        BLOCK_FIELD_IS_OBJECT => {
            let remaining = crate::refcount::REFCOUNTS.release(object);
            if remaining == 0 {
                crate::refcount::deallocate_tracked_object(object, "block_object_dispose");
            }
        }
        BLOCK_FIELD_IS_BLOCK => {
            shim_block_release(object);
        }
        _ => {
            // __block byref or unknown — no-op
        }
    }
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

    // ── imp_implementationWithBlock tests ──────────────────────────

    #[test]
    fn imp_implementation_with_block_null() {
        assert_eq!(shim_imp_implementation_with_block(0), 0);
    }

    #[test]
    fn imp_implementation_with_block_stores() {
        // Create a fake global block with an invoke pointer
        #[repr(C)]
        struct FakeBlock {
            isa: u64,
            flags: u32,
            reserved: u32,
            invoke: u64,
            descriptor: u64,
        }
        let blk = FakeBlock {
            isa: global_block_class_addr(),
            flags: BLOCK_IS_GLOBAL,
            reserved: 0,
            invoke: 0xCAFE_1234,
            descriptor: 0,
        };
        let addr = &blk as *const _ as u64;
        let imp = shim_imp_implementation_with_block(addr);
        assert_ne!(imp, 0, "should return non-zero IMP");
        assert_eq!(imp, 0xCAFE_1234, "IMP should be the block's invoke pointer");
    }

    #[test]
    fn imp_get_block_returns_stored() {
        #[repr(C)]
        struct FakeBlock {
            isa: u64,
            flags: u32,
            reserved: u32,
            invoke: u64,
            descriptor: u64,
        }
        let blk = FakeBlock {
            isa: global_block_class_addr(),
            flags: BLOCK_IS_GLOBAL,
            reserved: 0,
            invoke: 0xCAFE_5678,
            descriptor: 0,
        };
        let addr = &blk as *const _ as u64;
        let imp = shim_imp_implementation_with_block(addr);
        let got = shim_imp_get_block(imp);
        // For global blocks, block_copy returns the same pointer
        assert_eq!(got, addr);
    }

    #[test]
    fn imp_get_block_unknown() {
        assert_eq!(shim_imp_get_block(0xDEAD_9999), 0);
    }

    #[test]
    fn imp_remove_block_removes() {
        #[repr(C)]
        struct FakeBlock {
            isa: u64,
            flags: u32,
            reserved: u32,
            invoke: u64,
            descriptor: u64,
        }
        let blk = FakeBlock {
            isa: global_block_class_addr(),
            flags: BLOCK_IS_GLOBAL,
            reserved: 0,
            invoke: 0xCAFE_ABCD,
            descriptor: 0,
        };
        let addr = &blk as *const _ as u64;
        let imp = shim_imp_implementation_with_block(addr);
        assert_eq!(shim_imp_remove_block(imp), 1);
        // After removal, get_block should return 0
        assert_eq!(shim_imp_get_block(imp), 0);
    }

    #[test]
    fn imp_remove_block_unknown() {
        assert_eq!(shim_imp_remove_block(0xDEAD_AAAA), 0);
    }

    // ── _Block_object_assign / _Block_object_dispose tests ─────────

    #[test]
    fn block_object_assign_null_dest_is_noop() {
        shim_block_object_assign(0, 0xBEEF, BLOCK_FIELD_IS_OBJECT); // must not crash
    }

    #[test]
    fn block_object_assign_null_object_is_noop() {
        let mut dest: u64 = 0;
        shim_block_object_assign(&mut dest as *mut u64 as u64, 0, BLOCK_FIELD_IS_OBJECT);
        assert_eq!(dest, 0);
    }

    #[test]
    fn block_object_assign_object_stores_and_retains() {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        assert_ne!(obj, 0);
        let mut dest: u64 = 0;
        shim_block_object_assign(&mut dest as *mut u64 as u64, obj, BLOCK_FIELD_IS_OBJECT);
        assert_eq!(dest, obj);
        // Object was retained — refcount should be > 0
        let count = crate::refcount::REFCOUNTS.count(obj);
        assert!(count >= 1);
        // Clean up
        crate::refcount::REFCOUNTS.release(obj);
        crate::pal_alloc().free(obj as *mut u8);
    }

    #[test]
    fn block_object_assign_byref_copies_to_heap() {
        // Build a minimal byref struct:
        // [isa:8][forwarding:8][flags:4][size:4][captured:8]
        let mut byref = [0u8; BYREF_MIN_SIZE];
        let byref_ptr = byref.as_mut_ptr() as u64;
        unsafe {
            // forwarding → self (stack copy)
            *((byref.as_mut_ptr().add(BYREF_FORWARDING_OFF)) as *mut u64) = byref_ptr;
            // size = BYREF_MIN_SIZE
            *((byref.as_mut_ptr().add(BYREF_SIZE_OFF)) as *mut u32) = BYREF_MIN_SIZE as u32;
        }
        let mut dest: u64 = 0;
        shim_block_object_assign(
            &mut dest as *mut u64 as u64,
            byref_ptr,
            BLOCK_FIELD_IS_BYREF,
        );
        assert_ne!(dest, 0);
        assert_ne!(dest, byref_ptr, "byref should be copied to heap");
        // Stack forwarding → heap
        let stack_fwd = unsafe { *((byref.as_ptr().add(BYREF_FORWARDING_OFF)) as *const u64) };
        assert_eq!(stack_fwd, dest);
        // Heap forwarding → itself
        let heap_fwd = unsafe { *((dest as usize + BYREF_FORWARDING_OFF) as *const u64) };
        assert_eq!(heap_fwd, dest);
        unsafe {
            libc::free(dest as *mut libc::c_void);
        }
    }

    #[test]
    fn block_object_dispose_null_is_noop() {
        shim_block_object_dispose(0, BLOCK_FIELD_IS_OBJECT); // must not crash
    }

    #[test]
    fn block_object_dispose_byref_is_noop() {
        shim_block_object_dispose(0xBEEF, BLOCK_FIELD_IS_BYREF); // must not crash
    }
}
