//! NSArray shim: immutable ordered collection of objects.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa          (u64 — pointer to NSArray class)
//!   offset  8: count        (u64 — number of elements)
//!   offset 16: elements_ptr (u64 — pointer to malloc'd id[] array)
//!
//! Total: 24 bytes per array object.

use crate::cfstring;

pub mod ops;

pub(crate) const NSARRAY_COUNT_OFF: usize = 8;
pub(crate) const NSARRAY_ELEMS_OFF: usize = 16;
const NSARRAY_SIZE: usize = 24;

static NSARRAY_FAST_ENUM_MUTATIONS: u64 = 0;

pub(crate) fn collection_object_has_retainable_lifetime(obj: u64) -> bool {
    obj != 0
        && obj >= 0x1_0000_0000
        && (obj & 0xF) == 0
        && (obj >> 63) == 0
        && objc_runtime::refcount::object_appears_heap_allocated(obj)
}

pub(crate) fn retain_collection_object(obj: u64) {
    if collection_object_has_retainable_lifetime(obj) {
        objc_runtime::shim_nsobject_retain(obj, 0);
    }
}

pub(crate) fn release_collection_object(obj: u64) {
    if collection_object_has_retainable_lifetime(obj) {
        objc_runtime::shim_nsobject_release(obj, 0);
    }
}

fn release_array_elements(elems: u64, count: usize) {
    if elems == 0 || count == 0 {
        return;
    }

    for i in 0..count {
        let item = unsafe { *((elems as usize + i * 8) as *const u64) };
        release_collection_object(item);
        unsafe {
            *((elems as usize + i * 8) as *mut u64) = 0;
        }
    }
}

fn make_items_buffer(items: &[u64]) -> u64 {
    if items.is_empty() {
        return 0;
    }

    let buf = crate::pal_alloc().malloc(items.len() * 8) as *mut u64;
    if buf.is_null() {
        return 0;
    }

    for (i, &item) in items.iter().enumerate() {
        unsafe {
            buf.add(i).write(item);
        }
        retain_collection_object(item);
    }

    buf as u64
}

fn init_nsarray_items(obj: u64, items: &[u64]) -> u64 {
    if obj == 0 {
        return 0;
    }

    let buf = make_items_buffer(items);

    unsafe {
        if *(obj as *const u64) == 0 {
            *(obj as *mut u64) = crate::nsobj::nsarray_class_addr();
        }

        let old_count = *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) as usize;
        let old_elems = *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64);
        if old_elems != 0 {
            release_array_elements(old_elems, old_count);
            crate::pal_alloc().free(old_elems as *mut u8);
        }

        *((obj as usize + NSARRAY_COUNT_OFF) as *mut u64) = items.len() as u64;
        *((obj as usize + NSARRAY_ELEMS_OFF) as *mut u64) = buf;
    }

    obj
}

pub(crate) fn collect_variadic_object_args(args: &[u64]) -> Vec<u64> {
    let mut items = Vec::new();
    for &arg in args {
        if arg == 0 {
            break;
        }
        items.push(arg);
    }
    items
}

/// Create an NSArray from a slice of object pointers.
pub fn make_nsarray(items: &[u64]) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSARRAY_SIZE) as u64;
    if obj == 0 { return 0; }
    init_nsarray_items(obj, items)
}

/// `+[NSArray arrayWithObjects:count:]`
///
/// Class factory: creates immutable array from a C array of object pointers.
/// objects is a pointer to `id objects[]`, cnt is the element count.
pub extern "C" fn shim_nsarray_array_with_objects_count(
    cls: u64,
    _cmd: u64,
    objects: u64,
    cnt: u64,
) -> u64 {
    if cls == 0 {
        return 0;
    }

    let items = if cnt == 0 || objects == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(objects as *const u64, cnt as usize) }
    };
    let obj = make_nsarray(items);

    log::debug!(
        "NSArray arrayWithObjects:count: → {:#x} ({} elements)",
        obj,
        cnt
    );
    obj
}

/// `+[NSArray arrayWithObjects:]`
///
/// Collects inline variadic object arguments until a nil terminator.
pub extern "C" fn shim_nsarray_array_with_objects(
    cls: u64,
    _cmd: u64,
    first: u64,
    second: u64,
    third: u64,
    fourth: u64,
    fifth: u64,
    sixth: u64,
    seventh: u64,
    eighth: u64,
) -> u64 {
    if cls == 0 {
        return 0;
    }

    let items = collect_variadic_object_args(&[
        first, second, third, fourth, fifth, sixth, seventh, eighth,
    ]);
    let obj = make_nsarray(&items);

    log::debug!(
        "NSArray arrayWithObjects: → {:#x} ({} variadic elements)",
        obj,
        items.len()
    );
    obj
}

/// `-[NSArray initWithObjects:]`
///
/// Initializes an allocated array instance from inline variadic object
/// arguments terminated by nil.
pub extern "C" fn shim_nsarray_init_with_objects(
    obj: u64,
    _cmd: u64,
    first: u64,
    second: u64,
    third: u64,
    fourth: u64,
    fifth: u64,
    sixth: u64,
    seventh: u64,
    eighth: u64,
) -> u64 {
    let items = collect_variadic_object_args(&[
        first, second, third, fourth, fifth, sixth, seventh, eighth,
    ]);
    let obj = init_nsarray_items(obj, &items);

    log::debug!(
        "NSArray initWithObjects: → {:#x} ({} variadic elements)",
        obj,
        items.len()
    );
    obj
}

/// `+[NSArray arrayWithObject:]`
///
/// Class factory: creates an immutable single-element array.
pub extern "C" fn shim_nsarray_array_with_object(cls: u64, _cmd: u64, object: u64) -> u64 {
    if cls == 0 || object == 0 {
        return 0;
    }

    make_nsarray(&[object])
}

/// `-[NSArray initWithArray:]` — initializes an allocated array instance
/// by copying all objects from the source array.
pub extern "C" fn shim_nsarray_init_with_array(obj: u64, _cmd: u64, source: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    if source == 0 {
        return init_nsarray_items(obj, &[]);
    }
    let count = shim_nsarray_count(source, 0) as usize;
    let elems = unsafe { *((source as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        return init_nsarray_items(obj, &[]);
    }
    let items: Vec<u64> = (0..count)
        .map(|i| unsafe { *((elems as usize + i * 8) as *const u64) })
        .collect();
    init_nsarray_items(obj, &items)
}

/// `-[NSArray count]` — returns the number of elements.
pub extern "C" fn shim_nsarray_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) }
}

/// `-[NSArray objectAtIndex:]` — returns the element at the given index.
///
/// Returns 0 (nil) if index is out of bounds (real NSArray throws
/// NSRangeException, but we can't throw ObjC exceptions yet).
pub extern "C" fn shim_nsarray_object_at_index(obj: u64, _cmd: u64, index: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    if index >= count {
        eprintln!(
            "NSArray objectAtIndex:{} beyond bounds [0..{})",
            index, count
        );
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    let val = unsafe { *((elems as usize + index as usize * 8) as *const u64) };
    val
}

/// `-[NSArray firstObject]` — returns the first element or nil.
pub extern "C" fn shim_nsarray_first_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    if count == 0 {
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *(elems as *const u64) }
}

/// `-[NSArray lastObject]` — returns the last element or nil.
pub extern "C" fn shim_nsarray_last_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    if count == 0 {
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *((elems as usize + (count as usize - 1) * 8) as *const u64) }
}

/// `-[NSArray containsObject:]` — linear scan comparing pointers.
///
/// For NSString elements, we also compare via UTF8String if pointers differ.
pub extern "C" fn shim_nsarray_contains_object(obj: u64, _cmd: u64, target: u64) -> u8 {
    if obj == 0 || target == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 || count == 0 {
        return 0;
    }

    // Get target's string representation for comparison
    let target_cstr = cfstring::extract_cfstring_cstr(target);

    for i in 0..count as usize {
        let elem = unsafe { *((elems + i as u64 * 8) as *const u64) };
        // Pointer equality
        if elem == target {
            return 1;
        }
        // String content equality if both are string-like
        if let Some(target_s) = &target_cstr {
            if let Some(elem_s) = cfstring::extract_cfstring_cstr(elem) {
                if elem_s == *target_s {
                    return 1;
                }
            }
        }
    }
    0
}

/// `-[NSArray description]` — returns a string representation.
///
/// We return a simple format like "(elem1, elem2, ...)".
pub extern "C" fn shim_nsarray_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };

    let mut parts = Vec::new();
    for i in 0..count as usize {
        if elems == 0 {
            parts.push("(null)".to_string());
        } else {
            let elem = unsafe { *((elems + i as u64 * 8) as *const u64) };
            if let Some(s) = cfstring::extract_cfstring_cstr(elem) {
                parts.push(format!("\"{}\"", s));
            } else {
                parts.push(format!("{:#x}", elem));
            }
        }
    }
    let desc = format!("({})", parts.join(", "));

    // Create an NSString from the description
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSArray countByEnumeratingWithState:objects:count:]`
///
/// Supports Objective-C fast enumeration (`for (id x in array)`).
pub extern "C" fn shim_nsarray_count_by_enumerating_with_state(
    obj: u64,
    _cmd: u64,
    state: u64,
    stackbuf: u64,
    len: u64,
) -> u64 {
    if obj == 0 || state == 0 || stackbuf == 0 || len == 0 {
        return 0;
    }

    let count = shim_nsarray_count(obj, 0) as usize;
    let index = unsafe { *(state as *const u64) as usize };
    if index >= count {
        return 0;
    }

    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }

    let batch = std::cmp::min(len as usize, count - index);
    for offset in 0..batch {
        let value = unsafe { *((elems as usize + (index + offset) * 8) as *const u64) };
        unsafe {
            *((stackbuf as usize + offset * 8) as *mut u64) = value;
        }
    }

    unsafe {
        *(state as *mut u64) = (index + batch) as u64;
        *((state as usize + 8) as *mut u64) = stackbuf;
        *((state as usize + 16) as *mut u64) = (&raw const NSARRAY_FAST_ENUM_MUTATIONS) as u64;
    }

    batch as u64
}

#[cfg(test)]
mod tests;

