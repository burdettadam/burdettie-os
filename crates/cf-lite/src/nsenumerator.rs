//! NSEnumerator — lightweight forward iterator for Foundation collections.
//!
//! Layout (24 bytes):
//!   offset  0: isa          (u64)  → NSEnumerator class addr
//!   offset  8: source       (u64)  → the collection being enumerated (NSArray or key-array)
//!   offset 16: index        (u64)  → current position

const NSENUM_SOURCE_OFF: usize = 8;
const NSENUM_INDEX_OFF: usize = 16;

/// `+[NSEnumerator alloc]` — internal; shouldn't be called directly,
/// but `objectEnumerator` / `keyEnumerator` create instances.
pub fn alloc_enumerator(source: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsenumerator_class_addr();
        *((obj as usize + NSENUM_SOURCE_OFF) as *mut u64) = source;
        // index starts at 0 (calloc zeroes it)
    }
    obj
}

/// `-[NSEnumerator nextObject]` — returns the next element or nil (0).
pub extern "C" fn shim_nsenumerator_next_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let source = unsafe { *((obj as usize + NSENUM_SOURCE_OFF) as *const u64) };
    let index = unsafe { *((obj as usize + NSENUM_INDEX_OFF) as *const u64) };
    if source == 0 {
        return 0;
    }

    // Read source count (offset 8 in NSArray layout)
    let count = unsafe { *((source as usize + 8) as *const u64) };
    if index >= count {
        return 0;
    }

    // Read element at index (NSArray: elements_ptr at offset 16)
    let elems = unsafe { *((source as usize + 16) as *const u64) };
    let element = unsafe { *((elems as usize + index as usize * 8) as *const u64) };

    // Advance index
    unsafe {
        *((obj as usize + NSENUM_INDEX_OFF) as *mut u64) = index + 1;
    }

    element
}

/// `-[NSEnumerator allObjects]` — returns an NSArray of remaining objects.
pub extern "C" fn shim_nsenumerator_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let source = unsafe { *((obj as usize + NSENUM_SOURCE_OFF) as *const u64) };
    let index = unsafe { *((obj as usize + NSENUM_INDEX_OFF) as *const u64) };
    if source == 0 {
        // Return empty array
        return crate::nsarray::shim_nsarray_array_with_objects_count(
            crate::nsobj::nsarray_class_addr(),
            0,
            0,
            0,
        );
    }

    let count = unsafe { *((source as usize + 8) as *const u64) };
    if index >= count {
        return crate::nsarray::shim_nsarray_array_with_objects_count(
            crate::nsobj::nsarray_class_addr(),
            0,
            0,
            0,
        );
    }

    let elems = unsafe { *((source as usize + 16) as *const u64) };
    let remaining = (count - index) as usize;
    let mut ptrs: Vec<u64> = Vec::with_capacity(remaining);
    for i in 0..remaining {
        let elem = unsafe { *((elems as usize + (index as usize + i) * 8) as *const u64) };
        ptrs.push(elem);
    }

    // Advance index to end
    unsafe {
        *((obj as usize + NSENUM_INDEX_OFF) as *mut u64) = count;
    }

    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        ptrs.as_ptr() as u64,
        ptrs.len() as u64,
    )
}

// ── NSArray objectEnumerator / NSDictionary keyEnumerator ────────────

/// `-[NSArray objectEnumerator]` — returns an NSEnumerator over the array's elements.
pub extern "C" fn shim_nsarray_object_enumerator(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    alloc_enumerator(obj)
}

/// `-[NSDictionary keyEnumerator]` — returns an NSEnumerator over the dictionary's keys.
/// The NSDictionary layout stores keys at offset 16 as an internal array-like buffer.
/// We build a temporary NSArray from the keys and enumerate that.
pub extern "C" fn shim_nsdictionary_key_enumerator(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Get the allKeys NSArray and enumerate it
    let all_keys = crate::nsdictionary::shim_nsdictionary_all_keys(obj, 0);
    alloc_enumerator(all_keys)
}
