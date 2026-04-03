//! IOKit C-level stubs for Linux.
//!
//! IOKit has no Linux equivalent.  These stubs return safe values:
//! open/close/release operations return `kIOReturnSuccess (0)` so callers
//! that check for errors can proceed; query operations return NULL or 0.
//! Battery / power management assertions return success with harmless IDs.

use crate::table::ShimTable;

pub struct IOKitStubs;

/// `kIOReturnUnsupported` — IOKit error code for "not supported on this platform".
const K_IO_RETURN_UNSUPPORTED: i32 = 0xe00002c7u32 as i32;
/// `kIOReturnSuccess` — IOKit success code.
const K_IO_RETURN_SUCCESS: i32 = 0;

impl IOKitStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── Service matching & lifecycle ────────────────────────
        register_shims!(table,
            ("_IOServiceGetMatchingService",
             shim_io_service_get_matching_service as *const () as u64,
             "IOServiceGetMatchingService → 0 (no match)"),
            ("_IOServiceGetMatchingServices",
             shim_io_service_get_matching_services as *const () as u64,
             "IOServiceGetMatchingServices → kIOReturnSuccess, empty iterator"),
            ("_IOServiceMatching",
             shim_io_service_matching as *const () as u64,
             "IOServiceMatching → NULL"),
            ("_IOServiceOpen",
             shim_io_service_open as *const () as u64,
             "IOServiceOpen → kIOReturnSuccess, synthetic port"),
            ("_IOServiceClose",
             shim_io_return_success as *const () as u64,
             "IOServiceClose → kIOReturnSuccess"),
            ("_IOObjectRelease",
             shim_io_return_success as *const () as u64,
             "IOObjectRelease → kIOReturnSuccess"),
            ("_IOObjectGetClass",
             shim_io_object_get_class as *const () as u64,
             "IOObjectGetClass → kIOReturnUnsupported"),
            ("_IOIteratorNext",
             shim_io_iterator_next as *const () as u64,
             "IOIteratorNext → 0 (end of iteration)"),
            ("_IOIteratorReset",
             shim_io_void_stub as *const () as u64,
             "IOIteratorReset → void stub")
        );

        // ── Registry entry ──────────────────────────────────────
        register_shims!(table,
            ("_IORegistryEntryCreateCFProperty",
             shim_io_registry_entry_create_cf_property as *const () as u64,
             "IORegistryEntryCreateCFProperty → NULL"),
            ("_IORegistryEntryGetName",
             shim_io_registry_entry_get_name as *const () as u64,
             "IORegistryEntryGetName → kIOReturnUnsupported"),
            ("_IORegistryEntryGetPath",
             shim_io_registry_entry_get_path as *const () as u64,
             "IORegistryEntryGetPath → kIOReturnUnsupported"),
            ("_IORegistryEntryCreateCFProperties",
             shim_io_registry_entry_create_cf_properties as *const () as u64,
             "IORegistryEntryCreateCFProperties → kIOReturnUnsupported"),
            ("_IORegistryGetRootEntry",
             shim_io_registry_get_root_entry as *const () as u64,
             "IORegistryGetRootEntry → 0")
        );

        // ── Connect / call ──────────────────────────────────────
        register_shims!(table,
            ("_IOConnectCallMethod",
             shim_io_connect_call_method as *const () as u64,
             "IOConnectCallMethod → kIOReturnUnsupported"),
            ("_IOConnectCallStructMethod",
             shim_io_connect_call_struct_method as *const () as u64,
             "IOConnectCallStructMethod → kIOReturnUnsupported"),
            ("_IOConnectCallScalarMethod",
             shim_io_connect_call_scalar_method as *const () as u64,
             "IOConnectCallScalarMethod → kIOReturnUnsupported")
        );

        // ── Power management assertions ─────────────────────────
        register_shims!(table,
            ("_IOPMAssertionCreateWithName",
             shim_iopm_assertion_create as *const () as u64,
             "IOPMAssertionCreateWithName → kIOReturnSuccess, assertionID=1"),
            ("_IOPMAssertionRelease",
             shim_io_return_success as *const () as u64,
             "IOPMAssertionRelease → kIOReturnSuccess"),
            ("_IOPMAssertionSetProperty",
             shim_io_return_success as *const () as u64,
             "IOPMAssertionSetProperty → kIOReturnSuccess"),
            ("_IOPMCopyFeatureDefault",
             shim_io_null_stub as *const () as u64,
             "IOPMCopyFeatureDefault → NULL"),
            ("_IOPMCopyBatteryInfo",
             shim_iopm_copy_battery_info as *const () as u64,
             "IOPMCopyBatteryInfo → kIOReturnUnsupported (no battery)")
        );

        // ── HID device & manager ────────────────────────────────
        register_shims!(table,
            ("_IOHIDManagerCreate",
             shim_io_null_stub as *const () as u64,
             "IOHIDManagerCreate → NULL"),
            ("_IOHIDManagerOpen",
             shim_io_return_success as *const () as u64,
             "IOHIDManagerOpen → kIOReturnSuccess"),
            ("_IOHIDManagerClose",
             shim_io_return_success as *const () as u64,
             "IOHIDManagerClose → kIOReturnSuccess"),
            ("_IOHIDDeviceCreate",
             shim_io_null_stub as *const () as u64,
             "IOHIDDeviceCreate → NULL"),
            ("_IOHIDDeviceOpen",
             shim_io_return_success as *const () as u64,
             "IOHIDDeviceOpen → kIOReturnSuccess"),
            ("_IOHIDDeviceClose",
             shim_io_return_success as *const () as u64,
             "IOHIDDeviceClose → kIOReturnSuccess")
        );

        log::debug!("IOKitStubs: registered 27 IOKit stubs");
    }
}

// ── Service matching ────────────────────────────────────────────────────────

/// `IOServiceGetMatchingService` → 0 (no match found).
pub extern "C" fn shim_io_service_get_matching_service(
    _master_port: u32, _matching: u64,
) -> u32 {
    0 // IO_OBJECT_NULL
}

/// `IOServiceGetMatchingServices` → kIOReturnSuccess, empty iterator (0).
pub extern "C" fn shim_io_service_get_matching_services(
    _master_port: u32, _matching: u64, existing: *mut u32,
) -> i32 {
    if !existing.is_null() {
        unsafe { *existing = 0; } // IO_OBJECT_NULL = empty iterator
    }
    K_IO_RETURN_SUCCESS
}

/// `IOServiceMatching` → NULL (can't create CFDictionary without CF runtime).
pub extern "C" fn shim_io_service_matching(_name: *const u8) -> u64 {
    0 // NULL
}

/// `IOServiceOpen` → kIOReturnSuccess with synthetic connect port.
pub extern "C" fn shim_io_service_open(
    _service: u32, _owning_task: u32, _type_: u32, connect: *mut u32,
) -> i32 {
    if !connect.is_null() {
        unsafe { *connect = 1; } // synthetic non-zero port handle
    }
    K_IO_RETURN_SUCCESS
}

// ── Generic stubs ───────────────────────────────────────────────────────────

/// Generic stub returning `kIOReturnSuccess`.
pub extern "C" fn shim_io_return_success(_a: u64) -> i32 {
    K_IO_RETURN_SUCCESS
}

/// Generic stub returning `kIOReturnUnsupported`.
pub extern "C" fn shim_io_return_unsupported(_a: u64) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// Generic stub returning NULL.
pub extern "C" fn shim_io_null_stub(_a: u64) -> u64 {
    0 // NULL
}

/// Generic void stub (e.g. IOIteratorReset).
pub extern "C" fn shim_io_void_stub(_a: u64) {}

// ── Object / iterator ───────────────────────────────────────────────────────

/// `IOObjectGetClass` → kIOReturnUnsupported.
pub extern "C" fn shim_io_object_get_class(
    _obj: u32, _class: *mut u8,
) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// `IOIteratorNext` → 0 (end of iteration, no more objects).
pub extern "C" fn shim_io_iterator_next(_iterator: u32) -> u32 {
    0 // IO_OBJECT_NULL
}

// ── Registry ────────────────────────────────────────────────────────────────

/// `IORegistryEntryCreateCFProperty` → NULL.
pub extern "C" fn shim_io_registry_entry_create_cf_property(
    _entry: u32, _key: u64, _allocator: u64, _options: u32,
) -> u64 {
    0 // NULL
}

/// `IORegistryEntryGetName` → kIOReturnUnsupported.
pub extern "C" fn shim_io_registry_entry_get_name(
    _entry: u32, _name: *mut u8,
) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// `IORegistryEntryGetPath` → kIOReturnUnsupported.
pub extern "C" fn shim_io_registry_entry_get_path(
    _entry: u32, _plane: *const u8, _path: *mut u8,
) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// `IORegistryEntryCreateCFProperties` → kIOReturnUnsupported.
pub extern "C" fn shim_io_registry_entry_create_cf_properties(
    _entry: u32, _properties: *mut u64, _allocator: u64, _options: u32,
) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// `IORegistryGetRootEntry` → 0 (no root entry on Linux).
pub extern "C" fn shim_io_registry_get_root_entry(_master_port: u32) -> u32 {
    0 // IO_OBJECT_NULL
}

// ── Connect / call ──────────────────────────────────────────────────────────

/// `IOConnectCallMethod` → kIOReturnUnsupported.
pub extern "C" fn shim_io_connect_call_method(
    _connection: u32,
    _selector: u32,
    _scalar_input: *const u64,
    _scalar_input_cnt: u32,
    _struct_input: *const u8,
    _struct_input_size: usize,
    _scalar_output: *mut u64,
    _scalar_output_cnt: *mut u32,
    _struct_output: *mut u8,
    _struct_output_size: *mut usize,
) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// `IOConnectCallStructMethod` → kIOReturnUnsupported.
pub extern "C" fn shim_io_connect_call_struct_method(
    _connection: u32,
    _selector: u32,
    _struct_input: *const u8,
    _struct_input_size: usize,
    _struct_output: *mut u8,
    _struct_output_size: *mut usize,
) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// `IOConnectCallScalarMethod` → kIOReturnUnsupported.
pub extern "C" fn shim_io_connect_call_scalar_method(
    _connection: u32,
    _selector: u32,
    _scalar_input: *const u64,
    _scalar_input_cnt: u32,
    _scalar_output: *mut u64,
    _scalar_output_cnt: *mut u32,
) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

// ── Power management ────────────────────────────────────────────────────────

/// `IOPMAssertionCreateWithName` → kIOReturnSuccess, assertionID = 1.
pub extern "C" fn shim_iopm_assertion_create(
    _assertion_type: u64,
    _assertion_level: i32,
    _assertion_name: u64,
    assertion_id: *mut u32,
) -> i32 {
    if !assertion_id.is_null() {
        unsafe { *assertion_id = 1; } // synthetic valid assertion ID
    }
    K_IO_RETURN_SUCCESS
}

/// `IOPMCopyBatteryInfo` → kIOReturnUnsupported (no battery representation).
pub extern "C" fn shim_iopm_copy_battery_info(_master_port: u32, _info: *mut u64) -> i32 {
    // Return kIOReturnUnsupported — callers should treat this as "no battery"
    // rather than an error, so emitting 0 (kIOReturnSuccess) with NULL info
    // would be misleading.
    K_IO_RETURN_UNSUPPORTED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_service_matching_returns_null() {
        let ptr = b"IOService\0";
        assert_eq!(shim_io_service_matching(ptr.as_ptr()), 0);
    }

    #[test]
    fn io_service_get_matching_service_returns_zero() {
        assert_eq!(shim_io_service_get_matching_service(0, 0), 0);
    }

    #[test]
    fn io_iterator_next_returns_zero() {
        assert_eq!(shim_io_iterator_next(1), 0);
    }

    #[test]
    fn io_return_unsupported_value() {
        assert_eq!(K_IO_RETURN_UNSUPPORTED, 0xe00002c7u32 as i32);
    }

    #[test]
    fn io_registry_entry_create_cf_property_null() {
        assert_eq!(shim_io_registry_entry_create_cf_property(1, 0, 0, 0), 0);
    }

    #[test]
    fn io_null_stub_returns_zero() {
        assert_eq!(shim_io_null_stub(42), 0);
    }

    #[test]
    fn iopm_assertion_create_unsupported() {
        let mut id: u32 = 0;
        // shim returns kIOReturnSuccess (0) and sets assertionID = 1
        assert_eq!(shim_iopm_assertion_create(0, 255, 0, &mut id), K_IO_RETURN_SUCCESS);
        assert_eq!(id, 1);
    }
}
