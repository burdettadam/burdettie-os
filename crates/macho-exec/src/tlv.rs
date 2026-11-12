//! Thread-Local Variable (TLV) initialization for in-process execution.
//!
//! After fixups rebase TLV descriptor thunk pointers to the host's
//! `_tlv_bootstrap`, that function aborts because dyld never registered
//! our mapped binary's TLV data. This module patches the thunk pointers
//! in mapped `__thread_vars` sections to use our own TLV getter, which
//! allocates per-thread storage using `pthread_key_create`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use macho_parse::ParsedMachO;
use macho_types::segment::SectionType;

use crate::MappedImage;

#[cfg(unix)]
mod tls {
    pub type TlvKey = libc::pthread_key_t;

    pub fn create_key(destructor: unsafe extern "C" fn(*mut libc::c_void)) -> Option<TlvKey> {
        let mut key: libc::pthread_key_t = 0;
        let result = unsafe { libc::pthread_key_create(&mut key, Some(destructor)) };
        if result == 0 {
            Some(key)
        } else {
            None
        }
    }

    pub fn get_specific(key: TlvKey) -> *mut u8 {
        unsafe { libc::pthread_getspecific(key) as *mut u8 }
    }

    pub fn set_specific(key: TlvKey, value: *mut u8) -> i32 {
        unsafe { libc::pthread_setspecific(key, value as *mut libc::c_void) }
    }
}

#[cfg(windows)]
mod tls {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub type TlvKey = usize;

    static NEXT_KEY: AtomicUsize = AtomicUsize::new(1);

    struct ThreadTlvBlocks {
        blocks: HashMap<TlvKey, *mut u8>,
    }

    impl Drop for ThreadTlvBlocks {
        fn drop(&mut self) {
            for (_key, block) in self.blocks.drain() {
                unsafe {
                    super::tlv_block_destructor(block.cast());
                }
            }
        }
    }

    thread_local! {
        static BLOCKS: RefCell<ThreadTlvBlocks> =
            RefCell::new(ThreadTlvBlocks { blocks: HashMap::new() });
    }

    pub fn create_key(_destructor: unsafe extern "C" fn(*mut libc::c_void)) -> Option<TlvKey> {
        Some(NEXT_KEY.fetch_add(1, Ordering::Relaxed))
    }

    pub fn get_specific(key: TlvKey) -> *mut u8 {
        BLOCKS.with(|blocks| {
            blocks
                .borrow()
                .blocks
                .get(&key)
                .copied()
                .unwrap_or(std::ptr::null_mut())
        })
    }

    pub fn set_specific(key: TlvKey, value: *mut u8) -> i32 {
        BLOCKS.with(|blocks| {
            let mut blocks = blocks.borrow_mut();
            if value.is_null() {
                blocks.blocks.remove(&key);
            } else {
                blocks.blocks.insert(key, value);
            }
        });
        0
    }
}

#[cfg(not(any(unix, windows)))]
mod tls {
    pub type TlvKey = usize;

    pub fn create_key(_destructor: unsafe extern "C" fn(*mut libc::c_void)) -> Option<TlvKey> {
        Some(0)
    }

    pub fn get_specific(_key: TlvKey) -> *mut u8 {
        std::ptr::null_mut()
    }

    pub fn set_specific(_key: TlvKey, _value: *mut u8) -> i32 {
        0
    }
}

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
    pthread_key: tls::TlvKey,
    /// Total TLV block size spanning all TLV sections.
    total_size: usize,
    /// Actual allocation size used for the per-thread TLV block.
    allocation_size: usize,
    /// Template chunks copied into the per-thread block on first access.
    template_chunks: Vec<TlvTemplateChunk>,
}

struct TlvTemplateChunk {
    offset: usize,
    data: Vec<u8>,
}

#[derive(Clone, Debug)]
struct TlvTemplateChunkSnapshot {
    offset: usize,
    len: usize,
}

#[derive(Clone)]
struct TlvImageSnapshot {
    total_size: usize,
    allocation_size: usize,
    template_chunks: Vec<TlvTemplateChunkSnapshot>,
}

fn snapshot_from_info(info: &TlvImageInfo) -> TlvImageSnapshot {
    TlvImageSnapshot {
        total_size: info.total_size,
        allocation_size: info.allocation_size,
        template_chunks: info
            .template_chunks
            .iter()
            .map(|chunk| TlvTemplateChunkSnapshot {
                offset: chunk.offset,
                len: chunk.data.len(),
            })
            .collect(),
    }
}

struct TlvDestructorRegistration {
    dtor: extern "C" fn(*mut u8),
    arg: usize,
}

/// Global registry mapping descriptor key values to per-image TLV info.
static TLV_IMAGE_REGISTRY: Mutex<Option<TlvImageRegistry>> = Mutex::new(None);

const TLV_GUARD_BYTES: usize = 0x1_0000;

struct TlvImageRegistry {
    /// Maps descriptor addresses → image TLV info.
    descriptors: HashMap<usize, Arc<TlvImageInfo>>,
    /// Immutable snapshot of image TLV metadata keyed by Arc pointee address.
    image_snapshots: HashMap<usize, TlvImageSnapshot>,
    /// Per-thread destructor registrations keyed by TLV block address.
    block_destructors: HashMap<usize, Vec<TlvDestructorRegistration>>,
}

impl TlvImageRegistry {
    fn new() -> Self {
        Self {
            descriptors: HashMap::new(),
            image_snapshots: HashMap::new(),
            block_destructors: HashMap::new(),
        }
    }
}

/// Destructor called when a thread exits — frees the TLV block.
unsafe extern "C" fn tlv_block_destructor(ptr: *mut libc::c_void) {
    if !ptr.is_null() {
        let block_addr = ptr as usize;
        let registrations = {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
            guard
                .as_mut()
                .and_then(|registry| registry.block_destructors.remove(&block_addr))
                .unwrap_or_default()
        };
        for registration in registrations.into_iter().rev() {
            (registration.dtor)(registration.arg as *mut u8);
        }
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
    let offset = d.offset;
    let descriptor_addr = desc as usize;

    let guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let Some(registry) = guard.as_ref() else {
        log::error!("tlv_get_addr: registry not initialized");
        return std::ptr::null_mut();
    };
    let Some(info) = registry.descriptors.get(&descriptor_addr).cloned() else {
        log::error!("tlv_get_addr: unknown descriptor {descriptor_addr:#x}");
        return std::ptr::null_mut();
    };
    let info_ptr = Arc::as_ptr(&info) as usize;
    let snapshot = registry.image_snapshots.get(&info_ptr).cloned();
    let pkey = info.pthread_key;
    let total_size = info.total_size;
    let allocation_size = info.allocation_size;

    // Drop lock before pthread operations
    drop(guard);

    let snapshot = snapshot.unwrap_or_else(|| {
        log::warn!(
            "tlv_get_addr: missing TLV image snapshot for descriptor={descriptor_addr:#x} image={info_ptr:#x}; synthesizing from live metadata"
        );
        snapshot_from_info(&info)
    });

    if total_size != snapshot.total_size || allocation_size != snapshot.allocation_size {
        log::error!(
            "tlv_get_addr: corrupted TLV image metadata descriptor={descriptor_addr:#x} image={info_ptr:#x} offset={offset} live_block_size={total_size} live_allocation_size={allocation_size} expected_block_size={} expected_allocation_size={}",
            snapshot.total_size,
            snapshot.allocation_size,
        );
        return std::ptr::null_mut();
    }

    // Check if this thread already has a TLV block
    let mut block = tls::get_specific(pkey);
    if block.is_null() {
        // Allocate zeroed block
        block = unsafe { libc::calloc(1, allocation_size) } as *mut u8;
        if block.is_null() {
            log::error!("tlv_get_addr: calloc({allocation_size}) failed");
            return std::ptr::null_mut();
        }
        if info.template_chunks.len() != snapshot.template_chunks.len() {
            log::error!(
                "tlv_get_addr: corrupted TLV template chunk count descriptor={descriptor_addr:#x} image={info_ptr:#x} live_count={} expected_count={}",
                info.template_chunks.len(),
                snapshot.template_chunks.len(),
            );
            unsafe { libc::free(block as *mut libc::c_void) };
            return std::ptr::null_mut();
        }
        // Copy TLV template ranges into the allocated block.
        for (index, chunk) in info.template_chunks.iter().enumerate() {
            let Some(expected_chunk) = snapshot.template_chunks.get(index) else {
                log::error!(
                    "tlv_get_addr: missing expected TLV template chunk descriptor={descriptor_addr:#x} image={info_ptr:#x} index={index}"
                );
                unsafe { libc::free(block as *mut libc::c_void) };
                return std::ptr::null_mut();
            };
            let chunk_len = chunk.data.len();
            if chunk.offset != expected_chunk.offset || chunk_len != expected_chunk.len {
                log::error!(
                    "tlv_get_addr: corrupted TLV template chunk metadata descriptor={descriptor_addr:#x} image={info_ptr:#x} index={index} live_offset={} live_len={} expected_offset={} expected_len={}",
                    chunk.offset,
                    chunk_len,
                    expected_chunk.offset,
                    expected_chunk.len,
                );
                unsafe { libc::free(block as *mut libc::c_void) };
                return std::ptr::null_mut();
            }
            if chunk.data.is_empty() {
                continue;
            }
            let Some(chunk_end) = chunk.offset.checked_add(chunk_len) else {
                log::error!(
                    "tlv_get_addr: template chunk overflow descriptor={descriptor_addr:#x} image={info_ptr:#x} index={index} offset={} len={}",
                    chunk.offset,
                    chunk_len
                );
                unsafe { libc::free(block as *mut libc::c_void) };
                return std::ptr::null_mut();
            };
            if chunk_end > total_size {
                log::error!(
                    "tlv_get_addr: template chunk out of range descriptor={descriptor_addr:#x} image={info_ptr:#x} index={index} offset={} len={} block_size={total_size} expected_offset={} expected_len={} expected_block_size={}",
                    chunk.offset,
                    chunk_len,
                    expected_chunk.offset,
                    expected_chunk.len,
                    snapshot.total_size,
                );
                unsafe { libc::free(block as *mut libc::c_void) };
                return std::ptr::null_mut();
            }
            unsafe {
                std::ptr::copy_nonoverlapping(
                    chunk.data.as_ptr(),
                    block.add(chunk.offset),
                    chunk_len,
                );
            }
        }
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(registry) = guard.as_mut() {
                registry
                    .block_destructors
                    .entry(block as usize)
                    .or_default();
            }
        }
        let set_result = tls::set_specific(pkey, block);
        if set_result != 0 {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(registry) = guard.as_mut() {
                registry.block_destructors.remove(&(block as usize));
            }
            log::error!("tlv_get_addr: pthread_setspecific({pkey}) failed with {set_result}");
            unsafe {
                libc::free(block as *mut libc::c_void);
            }
            return std::ptr::null_mut();
        }
    }

    if offset >= total_size {
        log::error!("tlv_get_addr: offset {offset} outside block_size {total_size}");
        return std::ptr::null_mut();
    }

    unsafe { block.add(offset) }
}

/// Register a destructor for a thread-local object backed by a patched TLV block.
///
/// Returns `true` when the object address could be associated with the current
/// thread's TLV block and the destructor was queued to run before that block is
/// freed on thread exit.
pub fn register_tlv_atexit(arg: *mut u8, dtor: extern "C" fn(*mut u8)) -> bool {
    let images_by_key = {
        let guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
        let Some(registry) = guard.as_ref() else {
            return false;
        };
        let mut images = HashMap::new();
        for info in registry.descriptors.values() {
            images
                .entry(info.pthread_key)
                .or_insert_with(|| Arc::clone(info));
        }
        images
    };

    let mut matched_block = None;
    let arg_addr = arg as usize;
    for info in images_by_key.into_values() {
        let block = tls::get_specific(info.pthread_key);
        if block.is_null() {
            continue;
        }
        let block_addr = block as usize;
        if arg.is_null() {
            match matched_block {
                Some(existing) if existing != block_addr => return false,
                Some(_) => continue,
                None => {
                    matched_block = Some(block_addr);
                    continue;
                }
            }
        }
        let Some(block_end) = block_addr.checked_add(info.allocation_size) else {
            continue;
        };
        if arg_addr >= block_addr && arg_addr < block_end {
            matched_block = Some(block_addr);
            break;
        }
    }

    let Some(block_addr) = matched_block else {
        return false;
    };

    let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let Some(registry) = guard.as_mut() else {
        return false;
    };
    registry
        .block_destructors
        .entry(block_addr)
        .or_default()
        .push(TlvDestructorRegistration {
            dtor,
            arg: arg as usize,
        });
    true
}

/// Patch TLV descriptor thunks in a mapped image to use our TLV getter.
///
/// Must be called after fixups are applied but before `protect()` makes
/// __DATA read-only. Finds `__thread_vars` sections, computes the TLV
/// block size from the full `__thread_data`/`__thread_bss` address span, allocates a pthread
/// key, and writes our thunk + key into each descriptor.
///
/// Returns the number of TLV descriptors patched, or 0 if the image
/// has no TLVs.
pub fn patch_tlv_thunks(image: &MappedImage, parsed: &ParsedMachO) -> usize {
    // Find __thread_vars, __thread_data, __thread_bss sections
    let thread_vars = parsed
        .sections
        .iter()
        .find(|s| s.section_type_name() == SectionType::ThreadLocalVariables);
    let Some(thread_vars) = thread_vars else {
        return 0; // No TLVs in this image
    };

    let tlv_sections: Vec<_> = parsed
        .sections
        .iter()
        .filter(|s| {
            matches!(
                s.section_type_name(),
                SectionType::ThreadLocalRegular | SectionType::ThreadLocalZeroFill
            )
        })
        .collect();
    let Some(block_base_addr) = tlv_sections.iter().map(|s| s.addr).min() else {
        return 0;
    };
    let block_end_addr = tlv_sections
        .iter()
        .map(|s| s.addr + s.size)
        .max()
        .unwrap_or(block_base_addr);
    let total_size = (block_end_addr - block_base_addr) as usize;
    if total_size == 0 {
        return 0;
    }
    let allocation_size = total_size.saturating_add(TLV_GUARD_BYTES);

    // Extract TLV template ranges from all __thread_data sections.
    // IMPORTANT: read from the mapped image, not the raw file data.
    // After fixups, __thread_data may contain rebased pointers (e.g.
    // Rust thread_local_dtor DTORS entries).  Raw file bytes carry
    // pre-fixup values that point into unmapped or wrong memory.
    let template_chunks: Vec<TlvTemplateChunk> = parsed
        .sections
        .iter()
        .filter(|s| s.section_type_name() == SectionType::ThreadLocalRegular)
        .filter_map(|s| {
            let sz = s.size as usize;
            // Find the mapped segment that contains this section.
            let seg = image
                .segments
                .iter()
                .find(|seg| s.addr >= seg.vmaddr && s.addr < seg.vmaddr + seg.size as u64)?;
            let section_offset_in_seg = (s.addr - seg.vmaddr) as usize;
            if section_offset_in_seg + sz > seg.size {
                log::error!(
                    "patch_tlv_thunks: __thread_data section at {:#x} extends past segment {} bounds",
                    s.addr,
                    seg.name
                );
                return None;
            }
            let src = unsafe { std::slice::from_raw_parts(seg.base.add(section_offset_in_seg), sz) };
            // Diagnostic: dump first 32 bytes of template as u64 values
            {
                let mut hex = String::new();
                for chunk in src.chunks(8).take(4) {
                    let val = u64::from_le_bytes(chunk.try_into().unwrap_or([0u8;8]));
                    hex.push_str(&format!(" {val:#018x}"));
                }
                log::info!(
                    "patch_tlv_thunks: __thread_data at seg '{}' off {:#x} sz={sz}:{hex}",
                    seg.name,
                    section_offset_in_seg,
                );
            }
            Some(TlvTemplateChunk {
                offset: (s.addr - block_base_addr) as usize,
                data: src.to_vec(),
            })
        })
        .collect();

    // Allocate a pthread key for this image
    let Some(pkey) = tls::create_key(tlv_block_destructor) else {
        log::error!("patch_tlv_thunks: failed to create TLV key");
        return 0;
    };

    // Register in global registry
    let image_info = Arc::new(TlvImageInfo {
        pthread_key: pkey,
        total_size,
        allocation_size,
        template_chunks,
    });
    let image_ptr = Arc::as_ptr(&image_info) as usize;
    let image_snapshot = snapshot_from_info(&image_info);

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
    let Some(seg) = seg else {
        log::error!(
            "patch_tlv_thunks: cannot find segment for __thread_vars at {tv_section_addr:#x}"
        );
        return 0;
    };

    let section_offset_in_seg = (tv_section_addr - seg.vmaddr) as usize;
    let mut patched = 0;

    for i in 0..desc_count {
        let desc_offset = section_offset_in_seg + i * desc_size;
        if desc_offset + desc_size > seg.size {
            break;
        }
        let desc_ptr = unsafe { seg.base.add(desc_offset) as *mut TlvDescriptor };
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
            let registry = guard.get_or_insert_with(TlvImageRegistry::new);
            registry
                .image_snapshots
                .entry(image_ptr)
                .or_insert_with(|| image_snapshot.clone());
            registry
                .descriptors
                .insert(desc_ptr as usize, Arc::clone(&image_info));
        }
        unsafe {
            (*desc_ptr).thunk = tlv_get_addr;
            (*desc_ptr).key = pkey as usize;
            // offset stays as-is (it's the offset within the TLV block).
        }
        patched += 1;
    }

    if patched > 0 {
        log::info!("patch_tlv_thunks: patched {patched} TLV descriptors (block_size={total_size})");
    }

    patched
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each test replaces the process-wide registry and must retain its entries
    // until its lookups, destructor callbacks, and cleanup are complete.
    static TLV_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn cleanup_test_key(pkey: libc::pthread_key_t) {
        unsafe {
            let block = libc::pthread_getspecific(pkey);
            libc::pthread_setspecific(pkey, std::ptr::null_mut());
            if !block.is_null() {
                libc::free(block);
            }
            libc::pthread_key_delete(pkey);
        }
        let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
        *guard = None;
    }

    #[test]
    fn tlv_get_addr_returns_distinct_offsets() {
        let _registry_guard = TLV_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // Set up a mock TLV with two descriptors sharing the same key
        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let image_info = Arc::new(TlvImageInfo {
            pthread_key: pkey,
            total_size: 16,
            allocation_size: 16,
            template_chunks: Vec::new(),
        });
        let mut desc0 = TlvDescriptor {
            thunk: tlv_get_addr,
            key: 0,
            offset: 0,
        };
        let mut desc1 = TlvDescriptor {
            thunk: tlv_get_addr,
            key: 0,
            offset: 8,
        };
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.insert(TlvImageRegistry::new());
            registry.descriptors.insert(
                (&mut desc0 as *mut TlvDescriptor) as usize,
                Arc::clone(&image_info),
            );
            registry.descriptors.insert(
                (&mut desc1 as *mut TlvDescriptor) as usize,
                Arc::clone(&image_info),
            );
        }

        let ptr0 = unsafe { tlv_get_addr(&mut desc0) };
        let ptr1 = unsafe { tlv_get_addr(&mut desc1) };

        assert!(!ptr0.is_null());
        assert!(!ptr1.is_null());
        // The two pointers should be 8 bytes apart (same block, different offsets)
        assert_eq!(unsafe { ptr1.offset_from(ptr0) }, 8);

        // Cleanup
        cleanup_test_key(pkey);
    }

    #[test]
    fn tlv_get_addr_copies_template() {
        let _registry_guard = TLV_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let image_info = Arc::new(TlvImageInfo {
            pthread_key: pkey,
            total_size: 16,
            allocation_size: 16,
            template_chunks: vec![TlvTemplateChunk {
                offset: 0,
                data: vec![42u8, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 0, 0, 0, 0],
            }],
        });
        let mut desc = TlvDescriptor {
            thunk: tlv_get_addr,
            key: 0,
            offset: 0,
        };
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.insert(TlvImageRegistry::new());
            registry
                .descriptors
                .insert((&mut desc as *mut TlvDescriptor) as usize, image_info);
        }
        let ptr = unsafe { tlv_get_addr(&mut desc) };
        assert!(!ptr.is_null());
        // Should have copied template data
        let val0 = unsafe { *(ptr as *const u64) };
        let val1 = unsafe { *((ptr as *const u64).add(1)) };
        assert_eq!(val0, 42);
        assert_eq!(val1, 99);

        // Guest-visible key values must remain untouched.
        assert_eq!(desc.key, 0);

        // Cleanup
        cleanup_test_key(pkey);
    }

    #[test]
    fn tlv_get_addr_ignores_guest_key_field() {
        let _registry_guard = TLV_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let image_info = Arc::new(TlvImageInfo {
            pthread_key: pkey,
            total_size: 32,
            allocation_size: 32,
            template_chunks: Vec::new(),
        });
        let mut desc = TlvDescriptor {
            thunk: tlv_get_addr,
            key: 1,
            offset: 16,
        };
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.insert(TlvImageRegistry::new());
            registry
                .descriptors
                .insert((&mut desc as *mut TlvDescriptor) as usize, image_info);
        }

        let ptr = unsafe { tlv_get_addr(&mut desc) };

        assert!(!ptr.is_null());
        assert_eq!(desc.key, 1);

        cleanup_test_key(pkey);
    }

    #[test]
    fn tlv_get_addr_copies_template_with_gap() {
        let _registry_guard = TLV_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let image_info = Arc::new(TlvImageInfo {
            pthread_key: pkey,
            total_size: 32,
            allocation_size: 32,
            template_chunks: vec![TlvTemplateChunk {
                offset: 16,
                data: vec![7u8, 0, 0, 0, 0, 0, 0, 0],
            }],
        });
        let mut desc = TlvDescriptor {
            thunk: tlv_get_addr,
            key: 0,
            offset: 16,
        };
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.insert(TlvImageRegistry::new());
            registry
                .descriptors
                .insert((&mut desc as *mut TlvDescriptor) as usize, image_info);
        }

        let ptr = unsafe { tlv_get_addr(&mut desc) };

        assert!(!ptr.is_null());
        assert_eq!(unsafe { *(ptr as *const u64) }, 7);

        cleanup_test_key(pkey);
    }

    #[test]
    fn tlv_atexit_runs_destructors_before_block_free() {
        let _registry_guard = TLV_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        static DTOR_VALUES: Mutex<Vec<u8>> = Mutex::new(Vec::new());

        extern "C" fn record_byte(arg: *mut u8) {
            let value = unsafe { *arg };
            DTOR_VALUES.lock().unwrap().push(value);
        }

        DTOR_VALUES.lock().unwrap().clear();

        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let image_info = Arc::new(TlvImageInfo {
            pthread_key: pkey,
            total_size: 8,
            allocation_size: 8,
            template_chunks: Vec::new(),
        });
        let mut desc = TlvDescriptor {
            thunk: tlv_get_addr,
            key: 0,
            offset: 0,
        };
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.insert(TlvImageRegistry::new());
            registry
                .descriptors
                .insert((&mut desc as *mut TlvDescriptor) as usize, image_info);
        }

        let ptr = unsafe { tlv_get_addr(&mut desc) };
        assert!(!ptr.is_null());
        unsafe {
            *ptr = 1;
            *ptr.add(1) = 2;
        }

        assert!(register_tlv_atexit(ptr, record_byte));
        assert!(register_tlv_atexit(unsafe { ptr.add(1) }, record_byte));

        unsafe {
            let block = libc::pthread_getspecific(pkey);
            libc::pthread_setspecific(pkey, std::ptr::null_mut());
            tlv_block_destructor(block);
            libc::pthread_key_delete(pkey);
        }

        assert_eq!(&*DTOR_VALUES.lock().unwrap(), &[2, 1]);

        let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
        *guard = None;
    }

    #[test]
    fn tlv_atexit_accepts_null_arg_for_single_live_block() {
        let _registry_guard = TLV_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        static NULL_ARG_CALLS: Mutex<usize> = Mutex::new(0);

        extern "C" fn record_null(arg: *mut u8) {
            assert!(arg.is_null());
            *NULL_ARG_CALLS.lock().unwrap() += 1;
        }

        *NULL_ARG_CALLS.lock().unwrap() = 0;

        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_block_destructor));
        }
        let image_info = Arc::new(TlvImageInfo {
            pthread_key: pkey,
            total_size: 8,
            allocation_size: 8,
            template_chunks: Vec::new(),
        });
        let mut desc = TlvDescriptor {
            thunk: tlv_get_addr,
            key: 0,
            offset: 0,
        };
        {
            let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
            let registry = guard.insert(TlvImageRegistry::new());
            registry.descriptors.insert(
                (&mut desc as *mut TlvDescriptor) as usize,
                Arc::clone(&image_info),
            );
            registry.image_snapshots.insert(
                Arc::as_ptr(&image_info) as usize,
                snapshot_from_info(&image_info),
            );
        }

        let ptr = unsafe { tlv_get_addr(&mut desc) };
        assert!(!ptr.is_null());
        assert!(register_tlv_atexit(std::ptr::null_mut(), record_null));

        unsafe {
            let block = libc::pthread_getspecific(pkey);
            libc::pthread_setspecific(pkey, std::ptr::null_mut());
            tlv_block_destructor(block);
            libc::pthread_key_delete(pkey);
        }

        assert_eq!(*NULL_ARG_CALLS.lock().unwrap(), 1);

        let mut guard = TLV_IMAGE_REGISTRY.lock().unwrap();
        *guard = None;
    }
}
