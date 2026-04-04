//! Extra CoreFoundation C-API stubs: mutable collections, UUID, byte-order,
//! property list, and CFSet.
//!
//! These bridge to existing NSMutable* and NS* shim objects where possible.
//! All functions are C-ABI compatible so the shim table can register them.

use crate::nsobj;

// ── CFMutableArray ───────────────────────────────────────────────────────────

/// `CFMutableArrayRef CFArrayCreateMutable(alloc, capacity, callbacks)`
pub extern "C" fn shim_cf_array_create_mutable(
    _alloc: u64, capacity: i64, _callbacks: u64,
) -> u64 {
    // +[NSMutableArray arrayWithCapacity:capacity]
    crate::nsmutablearray::shim_nsmutablearray_array_with_capacity(
        nsobj::nsmutablearray_class_addr(), 0, capacity.max(0) as u64,
    )
}

/// `void CFArrayAppendValue(CFMutableArrayRef theArray, const void *value)`
pub extern "C" fn shim_cf_array_append_value(array: u64, value: u64) {
    if array == 0 { return; }
    crate::nsmutablearray::shim_nsmutablearray_add_object(array, 0, value);
}

/// `void CFArrayInsertValueAtIndex(CFMutableArrayRef theArray, CFIndex idx, const void *value)`
pub extern "C" fn shim_cf_array_insert_value_at_index(array: u64, idx: i64, value: u64) {
    if array == 0 { return; }
    crate::nsmutablearray::shim_nsmutablearray_insert_object_at_index(array, 0, value, idx as u64);
}

/// `void CFArrayRemoveValueAtIndex(CFMutableArrayRef theArray, CFIndex idx)`
pub extern "C" fn shim_cf_array_remove_value_at_index(array: u64, idx: i64) {
    if array == 0 { return; }
    crate::nsmutablearray::shim_nsmutablearray_remove_object_at_index(array, 0, idx as u64);
}

/// `void CFArraySetValueAtIndex(CFMutableArrayRef theArray, CFIndex idx, const void *value)`
pub extern "C" fn shim_cf_array_set_value_at_index(array: u64, idx: i64, value: u64) {
    if array == 0 { return; }
    crate::nsmutablearray::shim_nsmutablearray_replace_object_at_index(array, 0, value, idx as u64);
}

/// `void CFArrayRemoveAllValues(CFMutableArrayRef theArray)`
pub extern "C" fn shim_cf_array_remove_all_values(array: u64) {
    if array == 0 { return; }
    crate::nsmutablearray::shim_nsmutablearray_remove_all_objects(array, 0);
}

/// `void CFArraySortValues(CFMutableArrayRef, CFRange, CFComparatorFunction, void *context)`
/// Stub: no-op (sorting requires a full CFComparatorFunction shim).
pub extern "C" fn shim_cf_array_sort_values(
    _array: u64, _range_loc: i64, _range_len: i64,
    _comparator: u64, _context: u64,
) {}

// ── CFMutableDictionary ──────────────────────────────────────────────────────

/// `CFMutableDictionaryRef CFDictionaryCreateMutable(alloc, capacity, keyCallBacks, valueCallBacks)`
pub extern "C" fn shim_cf_dictionary_create_mutable(
    _alloc: u64, capacity: i64, _key_cbs: u64, _val_cbs: u64,
) -> u64 {
    // +[NSMutableDictionary dictionaryWithCapacity:capacity]
    crate::nsmutabledict::shim_nsmutabledict_dictionary_with_capacity(
        nsobj::nsmutabledictionary_class_addr(), 0, capacity.max(0) as u64,
    )
}

/// `void CFDictionarySetValue(CFMutableDictionaryRef theDict, const void *key, const void *value)`
pub extern "C" fn shim_cf_dictionary_set_value(dict: u64, key: u64, value: u64) {
    if dict == 0 { return; }
    // -[dict setObject:value forKey:key]
    crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, value, key);
}

/// `void CFDictionaryAddValue(CFMutableDictionaryRef, key, value)` — same as SetValue.
pub extern "C" fn shim_cf_dictionary_add_value(dict: u64, key: u64, value: u64) {
    shim_cf_dictionary_set_value(dict, key, value);
}

/// `void CFDictionaryReplaceValue(CFMutableDictionaryRef, key, value)` — only if key exists.
pub extern "C" fn shim_cf_dictionary_replace_value(dict: u64, key: u64, value: u64) {
    if dict == 0 { return; }
    // Check existence: -[dict objectForKey:key]
    let existing = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key);
    if existing != 0 {
        shim_cf_dictionary_set_value(dict, key, value);
    }
}

/// `void CFDictionaryRemoveValue(CFMutableDictionaryRef theDict, const void *key)`
pub extern "C" fn shim_cf_dictionary_remove_value(dict: u64, key: u64) {
    if dict == 0 { return; }
    crate::nsmutabledict::shim_nsmutabledict_remove_object_for_key(dict, 0, key);
}

/// `void CFDictionaryRemoveAllValues(CFMutableDictionaryRef theDict)`
pub extern "C" fn shim_cf_dictionary_remove_all_values(dict: u64) {
    if dict == 0 { return; }
    crate::nsmutabledict::shim_nsmutabledict_remove_all_objects(dict, 0);
}

// ── CFSet / CFMutableSet ─────────────────────────────────────────────────────

/// `CFSetRef CFSetCreate(alloc, values, count, callbacks)`
pub extern "C" fn shim_cf_set_create(
    _alloc: u64, values: *const u64, count: i64, _callbacks: u64,
) -> u64 {
    let objects = if values.is_null() || count <= 0 { 0 } else { values as u64 };
    let cnt = count.max(0) as u64;
    crate::nsset::shim_nsset_with_objects_count(nsobj::nsset_class_addr(), 0, objects, cnt)
}

/// `CFSetRef CFSetGetTypeID()` — returns 18 (a plausible CFSetTypeID).
pub extern "C" fn shim_cf_set_get_type_id() -> u64 { 18 }

/// `CFIndex CFSetGetCount(CFSetRef theSet)`
pub extern "C" fn shim_cf_set_get_count(set: u64) -> i64 {
    if set == 0 { return 0; }
    crate::nsset::shim_nsset_count(set, 0) as i64
}

/// `Boolean CFSetContainsValue(CFSetRef theSet, const void *value)`
pub extern "C" fn shim_cf_set_contains_value(set: u64, value: u64) -> u8 {
    if set == 0 { return 0; }
    let result = crate::nsset::shim_nsset_contains_object(set, 0, value);
    if result != 0 { 1 } else { 0 }
}

/// `void CFSetGetValues(CFSetRef theSet, const void **values)` — copies all pointers.
pub extern "C" fn shim_cf_set_get_values(set: u64, values: *mut u64) {
    if set == 0 || values.is_null() { return; }
    let arr = crate::nsset::shim_nsset_all_objects(set, 0);
    if arr == 0 { return; }
    let count = unsafe { *((arr as usize + 8) as *const u64) } as usize;
    let elems_ptr = unsafe { *((arr as usize + 16) as *const u64) };
    if elems_ptr == 0 { return; }
    unsafe { std::ptr::copy_nonoverlapping(elems_ptr as *const u64, values, count); }
}

/// `CFMutableSetRef CFSetCreateMutable(alloc, capacity, callbacks)`
pub extern "C" fn shim_cf_set_create_mutable(
    _alloc: u64, capacity: i64, _callbacks: u64,
) -> u64 {
    crate::nsmutableset::shim_nsmutableset_set_with_capacity(
        nsobj::nsmutableset_class_addr(), 0, capacity.max(0) as u64,
    )
}

/// `void CFSetAddValue(CFMutableSetRef theSet, const void *value)`
pub extern "C" fn shim_cf_set_add_value(set: u64, value: u64) {
    if set == 0 { return; }
    crate::nsmutableset::shim_nsmutableset_add_object(set, 0, value);
}

/// `void CFSetRemoveValue(CFMutableSetRef theSet, const void *value)`
pub extern "C" fn shim_cf_set_remove_value(set: u64, value: u64) {
    if set == 0 { return; }
    crate::nsmutableset::shim_nsmutableset_remove_object(set, 0, value);
}

/// `void CFSetRemoveAllValues(CFMutableSetRef theSet)`
pub extern "C" fn shim_cf_set_remove_all_values(set: u64) {
    if set == 0 { return; }
    crate::nsmutableset::shim_nsmutableset_remove_all_objects(set, 0);
}

// ── CFUUID ───────────────────────────────────────────────────────────────────

/// `CFUUIDRef CFUUIDCreate(CFAllocatorRef alloc)` — new random UUID (alias NSUUID).
pub extern "C" fn shim_cf_uuid_create(_alloc: u64) -> u64 {
    // +[NSUUID UUID]
    crate::nsuuid::shim_nsuuid_uuid(nsobj::nsuuid_class_addr(), 0)
}

/// `CFStringRef CFUUIDCreateString(alloc, theUUID)` — UUID → string representation.
pub extern "C" fn shim_cf_uuid_create_string(_alloc: u64, uuid: u64) -> u64 {
    if uuid == 0 { return 0; }
    // -[uuid UUIDString]
    crate::nsuuid::shim_nsuuid_uuid_string(uuid, 0)
}

/// `CFUUIDRef CFUUIDCreateFromString(alloc, uuidStr)` — parse UUID string.
pub extern "C" fn shim_cf_uuid_create_from_string(_alloc: u64, uuid_str: u64) -> u64 {
    if uuid_str == 0 { return 0; }
    // alloc + initWithUUIDString:
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    crate::nsuuid::shim_nsuuid_init_with_uuid_string(obj, 0, uuid_str)
}

/// `CFUUIDBytes CFUUIDGetUUIDBytes(theUUID)` — this returns a 16-byte struct.
/// Stub: returns 0 (the UUID struct is passed by hidden pointer in the ABI anyway).
pub extern "C" fn shim_cf_uuid_get_uuid_bytes(_uuid: u64) -> u64 {
    // Real CFUUIDBytes is a 16-byte struct returned via pointer; stub with 0.
    0
}

/// `CFUUIDRef CFUUIDCreateFromUUIDBytes(alloc, bytes)` — construct UUID from 16 raw bytes.
pub extern "C" fn shim_cf_uuid_create_from_uuid_bytes(_alloc: u64, _bytes_lo: u64, _bytes_hi: u64) -> u64 {
    // Stub: create a new random UUID (we don't have the full 16-byte argument unpacking)
    shim_cf_uuid_create(0)
}

// ── CFByteOrder ──────────────────────────────────────────────────────────────

/// `CFByteOrder CFByteOrderGetCurrent()` → 1 (CFByteOrderLittleEndian on ARM64/x86_64).
pub extern "C" fn shim_cf_byte_order_get_current() -> i32 { 1 }

/// `uint16_t CFSwapInt16BigToHost(uint16_t)`
pub extern "C" fn shim_cf_swap_int16_big_to_host(arg: u16) -> u16 { u16::from_be(arg) }
/// `uint32_t CFSwapInt32BigToHost(uint32_t)`
pub extern "C" fn shim_cf_swap_int32_big_to_host(arg: u32) -> u32 { u32::from_be(arg) }
/// `uint64_t CFSwapInt64BigToHost(uint64_t)`
pub extern "C" fn shim_cf_swap_int64_big_to_host(arg: u64) -> u64 { u64::from_be(arg) }

/// `uint16_t CFSwapInt16HostToBig(uint16_t)`
pub extern "C" fn shim_cf_swap_int16_host_to_big(arg: u16) -> u16 { arg.to_be() }
/// `uint32_t CFSwapInt32HostToBig(uint32_t)`
pub extern "C" fn shim_cf_swap_int32_host_to_big(arg: u32) -> u32 { arg.to_be() }
/// `uint64_t CFSwapInt64HostToBig(uint64_t)`
pub extern "C" fn shim_cf_swap_int64_host_to_big(arg: u64) -> u64 { arg.to_be() }

/// `uint16_t CFSwapInt16LittleToHost(uint16_t)` — no-op on little-endian.
pub extern "C" fn shim_cf_swap_int16_little_to_host(arg: u16) -> u16 { arg }
/// `uint32_t CFSwapInt32LittleToHost(uint32_t)` — no-op on little-endian.
pub extern "C" fn shim_cf_swap_int32_little_to_host(arg: u32) -> u32 { arg }
/// `uint64_t CFSwapInt64LittleToHost(uint64_t)` — no-op on little-endian.
pub extern "C" fn shim_cf_swap_int64_little_to_host(arg: u64) -> u64 { arg }

/// `uint16_t CFSwapInt16HostToLittle(uint16_t)` — no-op on little-endian.
pub extern "C" fn shim_cf_swap_int16_host_to_little(arg: u16) -> u16 { arg }
/// `uint32_t CFSwapInt32HostToLittle(uint32_t)` — no-op on little-endian.
pub extern "C" fn shim_cf_swap_int32_host_to_little(arg: u32) -> u32 { arg }
/// `uint64_t CFSwapInt64HostToLittle(uint64_t)` — no-op on little-endian.
pub extern "C" fn shim_cf_swap_int64_host_to_little(arg: u64) -> u64 { arg }

/// `double CFConvertDoubleSwappedToHost(CFSwappedFloat64)` — big-endian f64 → host.
pub extern "C" fn shim_cf_convert_float64_swapped_to_host(swapped: u64) -> f64 {
    f64::from_bits(u64::from_be(swapped))
}

/// `CFSwappedFloat32 CFConvertFloat32HostToSwapped(Float32)` — host f32 → big-endian bits.
pub extern "C" fn shim_cf_convert_float32_host_to_swapped(val: f32) -> u32 {
    val.to_bits().to_be()
}

// ── CFPropertyList (stubs) ───────────────────────────────────────────────────

/// `CFPropertyListRef CFPropertyListCreateWithData(alloc, data, options, format, error)`
pub extern "C" fn shim_cf_property_list_create_with_data(
    _alloc: u64, data: u64, options: u64, format: *mut i32, error: *mut u64,
) -> u64 {
    if data == 0 { return 0; }
    // Delegate to +[NSPropertyListSerialization propertyListWithData:options:format:error:]
    crate::nspropertylistserialization::shim_plist_with_data(
        nsobj::nspropertylistserialization_class_addr(), 0,
        data, options, format as u64, error as u64,
    )
}

/// `CFDataRef CFPropertyListCreateData(alloc, propertyList, format, options, error)`
pub extern "C" fn shim_cf_property_list_create_data(
    _alloc: u64, plist: u64, format: i64, options: u64, error: *mut u64,
) -> u64 {
    if plist == 0 { return 0; }
    // Delegate to +[NSPropertyListSerialization dataWithPropertyList:format:options:error:]
    crate::nspropertylistserialization::shim_plist_data_with_object(
        nsobj::nspropertylistserialization_class_addr(), 0,
        plist, format as u64, options, error as u64,
    )
}

/// `Boolean CFPropertyListIsValid(CFPropertyListRef plist, CFPropertyListFormat format)`
pub extern "C" fn shim_cf_property_list_is_valid(_plist: u64, _format: i32) -> u8 {
    if _plist == 0 { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_order_little_endian() {
        assert_eq!(shim_cf_byte_order_get_current(), 1);
    }

    #[test]
    fn swap_int32_big_to_host() {
        let be = 0x01020304u32.swap_bytes(); // big-endian representation of 0x01020304
        assert_eq!(shim_cf_swap_int32_big_to_host(be), 0x01020304);
    }

    #[test]
    fn swap_int16_roundtrip() {
        let orig: u16 = 0xABCD;
        let swapped = shim_cf_swap_int16_host_to_big(orig);
        let back = shim_cf_swap_int16_big_to_host(swapped);
        assert_eq!(orig, back);
    }

    #[test]
    fn swap_int64_roundtrip() {
        let orig: u64 = 0x0102030405060708;
        let swapped = shim_cf_swap_int64_host_to_big(orig);
        let back = shim_cf_swap_int64_big_to_host(swapped);
        assert_eq!(orig, back);
    }

    #[test]
    fn little_to_host_is_identity() {
        assert_eq!(shim_cf_swap_int32_little_to_host(0xDEADBEEF), 0xDEADBEEF);
    }

    #[test]
    fn cf_property_list_is_valid_null_returns_false() {
        assert_eq!(shim_cf_property_list_is_valid(0, 0), 0);
    }
}
