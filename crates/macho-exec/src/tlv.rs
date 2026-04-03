//! Thread-Local Variable (TLV) initialization for in-process execution.
//!
//! After fixups rebase TLV descriptor thunk pointers to the host's
//! `_tlv_bootstrap`, that function aborts because dyld never registered
//! our mapped binary's TLV data. This module patches the thunk pointers
//! in mapped `__thread_vars` sections to use our own TLV getter, which
//! allocates per-thread storage using `pthread_key_create`.

use std::collections::HashMap;
use std::sync::Mutex;

use macho_parse::ParsedMachO;
use macho_types::segment::SectionType;

use crate::MappedImage;

/// TLV descriptor layout matching the Mach-O ABI (3 pointers, 24 bytes on arm64).
#[repr(C)]
struct TlvDescriptor {
    thunk: unsafe extern "C" fn(*mut TlvDescriptor) -> *mut u8,
    key: usize,
    offset: usize,
}

/// Per-image TLV metadata: total block size and optional template data.
struct TlvImageInfo {
    /// pthread key for per-thread storage.
    pthread_key: libc::pthread_key_t,
    /// Total TLV block size (__thread_data + __thread_bss).
    total_size: usize,
    /// Template data from __thread_data (copied on first access per thread).
    /// Empty if all TLVs are zero-initialized (BSS only).
    template: Vec<u8>,
}

/// Global registry mapping descriptor key values to per-image TLV info.
static TLV_IMAGE_REGISTRY: Mutex<Option<TlvImageRegistry>> = Mutex::new(None);

struct TlvImageRegistry {
    /// Maps key value (written into descriptors) → image TLV info.
    images: HashMap<usize, TlvImageInfo>,
    next_key: usize,
}

impl TlvImageRegistry {
    fn new() -> Self {
        Self {
            images: HashMap::new(),
            next_key: 1,
        }
    }
}

/// Destructor called when a thread exits — frees the TLV block.
unsafe extern "C" fn tlv_block_destructor(ptr: *mut libc::c_void) {
    if !ptr.is_null() {
        unsafe { libc::free(ptr) };
    }
}

/// TLV thunk: returns a pointer to the current thread's copy of a
/// thread-local variable. Allocates and initializes the per-thread
/// TLV block on first access.
///
/// # Safety
///
/// Called from guest code via function pointer. `desc` must point to
/// a valid TLV descriptor in mapped memory with a key value that was
/// registered via `patch_tlv_thunks`.
unsafe extern "C" fn tlv_get_addr(desc: *mut TlvDescriptor) -> *mut u8 {
    let d = unsafe { &*desc };
    let key_val = d.key;
    let offset = d.offset;

    let guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let registry = match guard.as_ref() {
        Some(r) => r,
        None => {
            log::error!("tlv_get_addr: registry not initialized");
            return std::ptr::null_mut();
        }
    };
    let info = match registry.images.get(&key_val) {
        Some(i) => i,
        None => {
            log::error!("tlv_get_addr: unknown key {key_val}");
            return std::ptr::null_mut();
        }
    };
    let pkey = info.pthread_key;
    let total_size = info.total_size;
    let template_ptr = if info.template.is_empty() {
        std::ptr::null()
    } else {
        info.template.as_ptr()
    };
    let template_len = info.template.len();

    // Drop lock before pthread operations
    drop(guard);

    // Check if this thread already has a TLV block
    let mut block = unsafe { libc::pthread_getspecific(pkey) } as *mut u8;
    if block.is_null() {
        // Allocate zeroed block
        block = unsafe { libc::calloc(1, total_size) } as *mut u8;
        if block.is_null() {
            log::error!("tlv_get_addr: calloc({total_size}) failed");
            return std::ptr::null_mut();
        }
        // Copy template data if present
        if !template_ptr.is_null() && template_len > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(template_ptr, block, template_len);
            }
        }
        unsafe {
            libc::pthread_setspecific(pkey, block as *mut libc::c_void);
        }
    }

    unsafe { block.add(offset) }
}

/// Patch TLV descriptor thunks in a mapped image to use our TLV getter.
///
/// Must be called after fixups are applied but before `protect()` makes
/// __DATA read-only. Finds `__thread_vars` sections, computes the TLV
/// block size from `__thread_data` + `__thread_bss`, allocates a pthread
/// key, and writes our thunk + key into each descriptor.
///
/// Returns the number of TLV descriptors patched, or 0 if the image
/// has no TLVs.
pub fn patch_tlv_thunks(image: &MappedImage, parsed: &ParsedMachO, data: &[u8]) -> usize {
    // Find __thread_vars, __thread_data, __thread_bss sections
    let thread_vars = parsed
        .sections
        .iter()
        .find(|s| s.section_type_name() == SectionType::ThreadLocalVariables);
    let thread_vars = match thread_vars {
        Some(tv) => tv,
        None => return 0, // No TLVs in this image
    };

    // Compute total TLV block size
    let thread_data_size = parsed
        .sections
        .iter()
        .find(|s| s.section_type_name() == SectionType::ThreadLocalRegular)
        .map(|s| s.size as usize)
        .unwrap_or(0);
    let thread_bss_size = parsed
        .sections
        .iter()
        .find(|s| s.section_type_name() == SectionType::ThreadLocalZeroFill)
        .map(|s| s.size as usize)
        .unwrap_or(0);
    let total_size = thread_data_size + thread_bss_size;
    if total_size == 0 {
        return 0;
    }

    // Extract template data from __thread_data (if any)
    let template: Vec<u8> = parsed
        .sections
        .iter()
        .find(|s| s.section_type_name() == SectionType::ThreadLocalRegular)
        .and_then(|s| {
            let off = s.offset as usize;
            let sz = s.size as usize;
            data.get(off..off + sz).map(|d| d.to_vec())
        })
        .unwrap_or_default();

    // Allocate a pthread key for this image
    let mut pkey: libc::pthread_key_t = 0;
    unsafe {
        libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
    }

    // Register in global registry
    let key_val = {
        let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
        let registry = guard.get_or_insert_with(TlvImageRegistry::new);
        let kv = registry.next_key;
        registry.next_key += 1;
        registry.images.insert(
            kv,
            TlvImageInfo {
                pthread_key: pkey,
                total_size,
                template,
            },
        );
        kv
    };

    // Find the __thread_vars section in mapped memory and patch descriptors
    let tv_section_addr = thread_vars.addr;
    let tv_section_size = thread_vars.size as usize;
    let desc_size = 24; // 3 × 8 bytes on 64-bit
    let desc_count = tv_section_size / desc_size;

    // Find which segment contains __thread_vars
    let seg = image
        .segments
        .iter()
        .find(|s| tv_section_addr >= s.vmaddr && tv_section_addr < s.vmaddr + s.size as u64);
    let seg = match seg {
        Some(s) => s,
        None => {
            log::error!(
                "patch_tlv_thunks: cannot find segment for __thread_vars at {tv_section_addr:#x}"
            );
            return 0;
        }
    };

    let section_offset_in_seg = (tv_section_addr - seg.vmaddr) as usize;
    let mut patched = 0;

    for i in 0..desc_count {
        let desc_offset = section_offset_in_seg + i * desc_size;
        if desc_offset + desc_size > seg.size {
            break;
        }
        let desc_ptr = unsafe { seg.base.add(desc_offset) as *mut TlvDescriptor };
        unsafe {
            (*desc_ptr).thunk = tlv_get_addr;
            (*desc_ptr).key = key_val;
            // offset stays as-is (it's the offset within the TLV block)
        }
        patched += 1;
    }

    if patched > 0 {
        log::info!(
            "patch_tlv_thunks: patched {patched} TLV descriptors (block_size={total_size}, key={key_val})"
        );
    }

    patched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tlv_get_addr_returns_distinct_offsets() {
        // Set up a mock TLV with two descriptors sharing the same key
        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let key_val = {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.get_or_insert_with(TlvImageRegistry::new);
            let kv = registry.next_key;
            registry.next_key += 1;
            registry.images.insert(
                kv,
                TlvImageInfo {
                    pthread_key: pkey,
                    total_size: 16,
                    template: Vec::new(),
                },
            );
            kv
        };

        let mut desc0 = TlvDescriptor {
            thunk: tlv_get_addr,
            key: key_val,
            offset: 0,
        };
        let mut desc1 = TlvDescriptor {
            thunk: tlv_get_addr,
            key: key_val,
            offset: 8,
        };

        let ptr0 = unsafe { tlv_get_addr(&mut desc0) };
        let ptr1 = unsafe { tlv_get_addr(&mut desc1) };

        assert!(!ptr0.is_null());
        assert!(!ptr1.is_null());
        // The two pointers should be 8 bytes apart (same block, different offsets)
        assert_eq!(unsafe { ptr1.offset_from(ptr0) }, 8);

        // Cleanup
        unsafe {
            let block = libc::pthread_getspecific(pkey);
            libc::free(block);
            libc::pthread_key_delete(pkey);
        }
    }

    #[test]
    fn tlv_get_addr_copies_template() {
        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let template = vec![42u8, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 0, 0, 0, 0];
        let key_val = {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.get_or_insert_with(TlvImageRegistry::new);
            let kv = registry.next_key;
            registry.next_key += 1;
            registry.images.insert(
                kv,
                TlvImageInfo {
                    pthread_key: pkey,
                    total_size: 16,
                    template,
                },
            );
            kv
        };

        let mut desc = TlvDescriptor {
            thunk: tlv_get_addr,
            key: key_val,
            offset: 0,
        };
        let ptr = unsafe { tlv_get_addr(&mut desc) };
        assert!(!ptr.is_null());
        // Should have copied template data
        let val0 = unsafe { *(ptr as *const u64) };
        let val1 = unsafe { *((ptr as *const u64).add(1)) };
        assert_eq!(val0, 42);
        assert_eq!(val1, 99);

        // Cleanup
        unsafe {
            let block = libc::pthread_getspecific(pkey);
            libc::free(block);
            libc::pthread_key_delete(pkey);
        }
    }
}
