//! IOKit shims for Linux — synthetic service registry backed by sysfs.
//!
//! Provides real behaviour for the most-used IOKit APIs:
//!
//! * **Service matching**: `IOServiceMatching` creates a tracked matching
//!   dictionary (opaque CFDictionary), `IOServiceGetMatchingService` /
//!   `IOServiceGetMatchingServices` look up the synthetic registry.
//! * **Registry properties**: `IORegistryEntryCreateCFProperty` /
//!   `IORegistryEntryCreateCFProperties` return data sourced from Linux
//!   sysfs (`/sys/class/power_supply/`, `/sys/class/dmi/`, etc.).
//! * **Power assertions**: `IOPMAssertionCreateWithName` tracks assertions
//!   with unique IDs; `IOPMAssertionRelease` releases them.
//! * **Power source info**: `IOPSCopyPowerSourcesInfo` / `IOPSCopyPowerSourcesList`
//!   / `IOPSGetPowerSourceDescription` report real battery status from sysfs.
//! * **Data symbols**: `kIOPMAssertionTypeNoIdleSleep`,
//!   `kIOPMAssertionTypePreventUserIdleSystemSleep`, `kIOMasterPortDefault`,
//!   `kIOMainPortDefault` are all provided.

use crate::table::ShimTable;
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

pub struct IOKitStubs;

/// `kIOReturnUnsupported` — IOKit error code for "not supported on this platform".
const K_IO_RETURN_UNSUPPORTED: i32 = 0xe00002c7u32 as i32;
/// `kIOReturnSuccess` — IOKit success code.
const K_IO_RETURN_SUCCESS: i32 = 0;
/// `kIOReturnNotFound` — IOKit error for entry not found.
const K_IO_RETURN_NOT_FOUND: i32 = 0xe00002beu32 as i32;

// ── Handle allocator ────────────────────────────────────────────────────────

/// Monotonic handle allocator for io_object_t / io_service_t / io_iterator_t.
/// Starts above 0x1000 to avoid confusion with small sentinel values.
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(0x1001);

fn alloc_handle() -> u32 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// ── Matching dictionary tracking ────────────────────────────────────────────

/// Maps matching dict handle → IOKit class name (e.g. "IOPMPowerSource").
static MATCHING_DICTS: Mutex<Option<HashMap<u64, String>>> = Mutex::new(None);

fn track_matching_dict(class_name: &str) -> u64 {
    let handle = alloc_handle() as u64;
    let mut guard = MATCHING_DICTS.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(handle, class_name.to_string());
    handle
}

fn lookup_matching_class(dict_handle: u64) -> Option<String> {
    MATCHING_DICTS
        .lock()
        .unwrap()
        .as_ref()?
        .get(&dict_handle)
        .cloned()
}

// ── Synthetic service registry ──────────────────────────────────────────────

/// Known IOKit classes that we can provide synthetic services for.
/// Each service has a fixed handle and a class name.
#[derive(Clone, Debug)]
struct SyntheticService {
    handle: u32,
    class_name: &'static str,
}

/// Registry of synthetic services, populated lazily from sysfs probes.
static SERVICE_REGISTRY: Mutex<Option<Vec<SyntheticService>>> = Mutex::new(None);

fn ensure_registry() -> Vec<SyntheticService> {
    let mut guard = SERVICE_REGISTRY.lock().unwrap();
    if let Some(ref services) = *guard {
        return services.clone();
    }

    let mut services = Vec::new();

    // Always provide AppleACPIPlatformExpert (root platform device)
    services.push(SyntheticService {
        handle: alloc_handle(),
        class_name: "AppleACPIPlatformExpert",
    });

    // IOPMPowerSource — battery, if /sys/class/power_supply/BAT* exists
    if has_sysfs_battery() {
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "IOPMPowerSource",
        });
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "AppleSmartBattery",
        });
    }

    // IOPlatformExpertDevice — always present (provides serial number, model)
    services.push(SyntheticService {
        handle: alloc_handle(),
        class_name: "IOPlatformExpertDevice",
    });

    // IODisplayConnect — display info if /sys/class/drm exists
    if std::path::Path::new("/sys/class/drm").exists() {
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "IODisplayConnect",
        });
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "IOFramebufferI2CInterface",
        });
    }

    // IOEthernetInterface if /sys/class/net/ has real NICs
    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Skip loopback and virtual devices
            if name_str != "lo" && !name_str.starts_with("veth") && !name_str.starts_with("docker")
            {
                services.push(SyntheticService {
                    handle: alloc_handle(),
                    class_name: "IOEthernetInterface",
                });
                break;
            }
        }
    }

    // IOBlockStorageDevice — if /sys/class/block/ has block devices
    if std::path::Path::new("/sys/class/block").exists() {
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "IOBlockStorageDevice",
        });
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "IOMediaBSDClient",
        });
    }

    // AppleACPICPU — CPU info from /sys/devices/system/cpu/
    if std::path::Path::new("/sys/devices/system/cpu/cpu0").exists() {
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "AppleACPICPU",
        });
    }

    // IOBlockStorageDevice — one per block device in /sys/block
    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Include real disks (sd*, vd*, nvme*, mmcblk*), skip loop/ram
            if name.starts_with("sd")
                || name.starts_with("vd")
                || name.starts_with("nvme")
                || name.starts_with("mmcblk")
            {
                services.push(SyntheticService {
                    handle: alloc_handle(),
                    class_name: "IOBlockStorageDevice",
                });
                // Also register as IOMedia for diskutil
                services.push(SyntheticService {
                    handle: alloc_handle(),
                    class_name: "IOMedia",
                });
                break; // One is enough for basic enumeration
            }
        }
    }

    // IONetworkInterface — if /sys/class/net has non-loopback interfaces
    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name != "lo" {
                services.push(SyntheticService {
                    handle: alloc_handle(),
                    class_name: "IONetworkInterface",
                });
                services.push(SyntheticService {
                    handle: alloc_handle(),
                    class_name: "IOEthernetInterface",
                });
                break;
            }
        }
    }

    // IOUSBHostDevice — if /sys/bus/usb/devices has entries
    #[cfg(target_os = "linux")]
    if std::path::Path::new("/sys/bus/usb/devices").exists() {
        services.push(SyntheticService {
            handle: alloc_handle(),
            class_name: "IOUSBHostDevice",
        });
    }

    *guard = Some(services.clone());

    // Register paths for IORegistryEntryFromPath lookups
    for s in &services {
        register_path(&format!("IOService:/{}", s.class_name), s.handle);
        register_path(&format!("IODeviceTree:/{}", s.class_name), s.handle);
    }

    services
}

fn find_service_by_class(class_name: &str) -> Option<u32> {
    ensure_registry()
        .iter()
        .find(|s| s.class_name == class_name)
        .map(|s| s.handle)
}

fn find_service_by_handle(handle: u32) -> Option<SyntheticService> {
    ensure_registry()
        .iter()
        .find(|s| s.handle == handle)
        .cloned()
}

// ── Iterator tracking ───────────────────────────────────────────────────────

/// Maps iterator handle → (vec of service handles, current index).
type ServiceIterators = HashMap<u32, (Vec<u32>, usize)>;
static ITERATORS: Mutex<Option<ServiceIterators>> = Mutex::new(None);

fn create_iterator(services: Vec<u32>) -> u32 {
    let handle = alloc_handle();
    let mut guard = ITERATORS.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(handle, (services, 0));
    handle
}

// ── Power assertion tracking ────────────────────────────────────────────────

static NEXT_ASSERTION_ID: AtomicU32 = AtomicU32::new(1);

/// Maps assertion ID → (type name, assertion name) for debugging/introspection.
static ACTIVE_ASSERTIONS: Mutex<Option<HashMap<u32, (String, String)>>> = Mutex::new(None);

// ── CFString allocation (self-contained, no cf-lite dependency) ─────────────

/// Allocate a simple CFString-compatible object from a Rust &str.
///
/// Layout: `[isa:8][len:8][chars_ptr:8][capacity:8][utf8 bytes + NUL]`
/// Total: 32 + len + 1 bytes.
fn alloc_cfstring(s: &str) -> u64 {
    let len = s.len();
    let ptr = unsafe { libc::calloc(1, 32 + len + 1) } as u64;
    if ptr == 0 {
        return 0;
    }
    let buf = ptr as *mut u8;
    unsafe {
        let isa = 0x1u64;
        std::ptr::copy_nonoverlapping(&isa as *const u64 as *const u8, buf, 8);
        let len_u64 = len as u64;
        std::ptr::copy_nonoverlapping(&len_u64 as *const u64 as *const u8, buf.add(8), 8);
        let chars_ptr = ptr + 32;
        std::ptr::copy_nonoverlapping(&chars_ptr as *const u64 as *const u8, buf.add(16), 8);
        std::ptr::copy_nonoverlapping(s.as_ptr(), buf.add(32), len);
        *buf.add(32 + len) = 0;
    }
    ptr
}

/// Allocate a CFNumber-compatible object holding an i64 value.
///
/// Layout: `[isa:8][value:8]` = 16 bytes.
fn alloc_cfnumber_i64(val: i64) -> u64 {
    let ptr = unsafe { libc::calloc(1, 16) } as u64;
    if ptr == 0 {
        return 0;
    }
    unsafe {
        let isa = 0x2u64; // sentinel ISA for number
        std::ptr::copy_nonoverlapping(&isa as *const u64 as *const u8, ptr as *mut u8, 8);
        std::ptr::copy_nonoverlapping(&val as *const i64 as *const u8, (ptr as *mut u8).add(8), 8);
    }
    ptr
}

/// Allocate a CFBoolean-like sentinel.
fn alloc_cfbool(val: bool) -> u64 {
    if val {
        1
    } else {
        0
    } // macOS kCFBooleanTrue/False are sentinel addresses
}

/// Allocate a CFDictionary-compatible mutable object.
///
/// Layout: `[isa:8][count:8][keys_ptr:8][vals_ptr:8][capacity:8]` = 40 bytes.
fn alloc_mutable_dict() -> u64 {
    let ptr = unsafe { libc::calloc(1, 40) } as u64;
    if ptr != 0 {
        unsafe {
            let isa = 0x3u64; // sentinel ISA for dict
            std::ptr::copy_nonoverlapping(&isa as *const u64 as *const u8, ptr as *mut u8, 8);
        }
    }
    ptr
}

/// Add a key-value pair to a mutable dict allocated by `alloc_mutable_dict`.
fn dict_set(dict: u64, key: u64, val: u64) {
    if dict == 0 || key == 0 {
        return;
    }
    unsafe {
        let base = dict as *mut u8;
        let count = *(base.add(8) as *const u64) as usize;
        let keys_ptr = *(base.add(16) as *const u64) as *mut u64;
        let vals_ptr = *(base.add(24) as *const u64) as *mut u64;

        let new_count = count + 1;
        let new_keys = libc::realloc(keys_ptr as _, new_count * 8) as *mut u64;
        let new_vals = libc::realloc(vals_ptr as _, new_count * 8) as *mut u64;
        if new_keys.is_null() || new_vals.is_null() {
            return;
        }

        *new_keys.add(count) = key;
        *new_vals.add(count) = val;

        *(base.add(8) as *mut u64) = new_count as u64;
        *(base.add(16) as *mut u64) = new_keys as u64;
        *(base.add(24) as *mut u64) = new_vals as u64;
    }
}

// ── sysfs helpers ───────────────────────────────────────────────────────────

/// Check if any battery exists in /sys/class/power_supply/.
fn has_sysfs_battery() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("BAT") {
                    return true;
                }
            }
        }
    }
    false
}

/// Read a sysfs attribute value as a trimmed string.
fn read_sysfs(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Read a sysfs attribute as an i64.
fn read_sysfs_i64(path: &str) -> Option<i64> {
    read_sysfs(path)?.parse().ok()
}

/// Find the first BATn directory under /sys/class/power_supply/.
#[cfg(target_os = "linux")]
fn first_battery_path() -> Option<String> {
    let entries = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("BAT") {
            return Some(format!("/sys/class/power_supply/{}", name_str));
        }
    }
    None
}

/// Build a CFDictionary of power source properties from sysfs.
fn build_power_source_dict() -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    #[cfg(target_os = "linux")]
    if let Some(bat_path) = first_battery_path() {
        // "Is Charging" / "Is Charged"
        let status = read_sysfs(&format!("{}/status", bat_path)).unwrap_or_default();
        let is_charging = status == "Charging";
        let is_charged = status == "Full";

        dict_set(
            dict,
            alloc_cfstring("Is Charging"),
            alloc_cfbool(is_charging),
        );
        dict_set(dict, alloc_cfstring("Is Charged"), alloc_cfbool(is_charged));
        dict_set(dict, alloc_cfstring("Is Present"), alloc_cfbool(true));

        // Current capacity / max capacity
        if let Some(now) = read_sysfs_i64(&format!("{}/energy_now", bat_path))
            .or_else(|| read_sysfs_i64(&format!("{}/charge_now", bat_path)))
        {
            dict_set(
                dict,
                alloc_cfstring("Current Capacity"),
                alloc_cfnumber_i64(now),
            );
        }
        if let Some(full) = read_sysfs_i64(&format!("{}/energy_full", bat_path))
            .or_else(|| read_sysfs_i64(&format!("{}/charge_full", bat_path)))
        {
            dict_set(
                dict,
                alloc_cfstring("Max Capacity"),
                alloc_cfnumber_i64(full),
            );
        }

        // Power source state
        if let Some(ac_online) = read_sysfs(&format!("{}/power_supply", bat_path)).or_else(|| {
            // Check AC adapter
            if std::path::Path::new("/sys/class/power_supply/AC/online").exists() {
                read_sysfs("/sys/class/power_supply/AC/online")
            } else {
                None
            }
        }) {
            let on_ac = ac_online.trim() == "1";
            dict_set(
                dict,
                alloc_cfstring("Power Source State"),
                alloc_cfstring(if on_ac { "AC Power" } else { "Battery Power" }),
            );
        }

        // Name
        if let Some(name) = read_sysfs(&format!("{}/model_name", bat_path))
            .or_else(|| read_sysfs(&format!("{}/manufacturer", bat_path)))
        {
            dict_set(dict, alloc_cfstring("Name"), alloc_cfstring(&name));
        }

        // Type
        dict_set(
            dict,
            alloc_cfstring("Type"),
            alloc_cfstring("InternalBattery"),
        );

        return dict;
    }

    // No battery — report AC power
    dict_set(dict, alloc_cfstring("Is Present"), alloc_cfbool(false));
    dict_set(
        dict,
        alloc_cfstring("Power Source State"),
        alloc_cfstring("AC Power"),
    );
    dict_set(dict, alloc_cfstring("Type"), alloc_cfstring("UPS"));
    dict
}

/// Build a properties dict for IOPlatformExpertDevice from Linux DMI/sysfs.
fn build_platform_expert_props() -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    // Serial number
    if let Some(serial) = read_sysfs("/sys/class/dmi/id/product_serial") {
        dict_set(
            dict,
            alloc_cfstring("IOPlatformSerialNumber"),
            alloc_cfstring(&serial),
        );
    } else {
        dict_set(
            dict,
            alloc_cfstring("IOPlatformSerialNumber"),
            alloc_cfstring("BURDETTIE0001"),
        );
    }

    // Hardware UUID
    if let Some(uuid) = read_sysfs("/sys/class/dmi/id/product_uuid") {
        dict_set(
            dict,
            alloc_cfstring("IOPlatformUUID"),
            alloc_cfstring(&uuid),
        );
    } else {
        dict_set(
            dict,
            alloc_cfstring("IOPlatformUUID"),
            alloc_cfstring("00000000-0000-0000-0000-000000000000"),
        );
    }

    // Model
    if let Some(model) = read_sysfs("/sys/class/dmi/id/product_name") {
        dict_set(dict, alloc_cfstring("model"), alloc_cfstring(&model));
    } else {
        dict_set(
            dict,
            alloc_cfstring("model"),
            alloc_cfstring("BurdettieOS Virtual Platform"),
        );
    }

    // Board ID
    if let Some(board_id) = read_sysfs("/sys/class/dmi/id/board_name") {
        dict_set(dict, alloc_cfstring("board-id"), alloc_cfstring(&board_id));
    }

    // Manufacturer
    if let Some(mfg) = read_sysfs("/sys/class/dmi/id/sys_vendor") {
        dict_set(dict, alloc_cfstring("manufacturer"), alloc_cfstring(&mfg));
    }

    dict
}

// ── Data symbols ────────────────────────────────────────────────────────────

/// `kIOMainPortDefault` / `kIOMasterPortDefault` — value 0 (MACH_PORT_NULL).
/// Guest code reads `*kIOMasterPortDefault` to get the master port value.
static K_IO_MASTER_PORT_DEFAULT: u32 = 0; // MACH_PORT_NULL

/// Power assertion type constants (CFStringRef data symbols).
/// These are static NUL-terminated byte arrays whose addresses are used
/// as CFString identity tokens.
macro_rules! io_const {
    ($name:ident, $value:literal) => {
        static $name: &[u8] = concat!($value, "\0").as_bytes();
    };
}

io_const!(IOPM_NO_IDLE_SLEEP, "NoIdleSleepAssertion");
io_const!(IOPM_NO_DISPLAY_SLEEP, "NoDisplaySleepAssertion");
io_const!(IOPM_PREVENT_USER_IDLE_SYSTEM, "PreventUserIdleSystemSleep");
io_const!(
    IOPM_PREVENT_USER_IDLE_DISPLAY,
    "PreventUserIdleDisplaySleep"
);
io_const!(IOPM_PREVENT_SYSTEM_SLEEP, "PreventSystemSleep");
io_const!(IOPM_EXTERNAL_MEDIA, "ExternalMedia");
io_const!(IOPM_NETWORK_CLIENT, "NetworkClientActive");

// IOKit plane name constants
io_const!(IO_SERVICE_PLANE, "IOService");
io_const!(IO_POWER_PLANE, "IOPower");
io_const!(IO_DEVICE_TREE_PLANE, "IODeviceTree");
io_const!(IO_USB_PLANE, "IOUSB");
io_const!(IO_FIREWIRE_PLANE, "IOFireWire");

// IOKit class name constants for service matching
io_const!(IO_USB_DEVICE_CLASS_NAME, "IOUSBDevice");
io_const!(IO_USB_INTERFACE_CLASS_NAME, "IOUSBInterface");
io_const!(IO_ETHERNET_INTERFACE_CLASS, "IOEthernetInterface");
io_const!(IO_BLOCK_STORAGE_DEVICE_CLASS, "IOBlockStorageDevice");

// IOKit HID key constants
io_const!(IO_HID_DEVICE_KEY, "IOHIDDevice");
io_const!(IO_HID_PRODUCT_KEY, "Product");
io_const!(IO_HID_VENDOR_ID_KEY, "VendorID");
io_const!(IO_HID_PRODUCT_ID_KEY, "ProductID");
io_const!(IO_HID_TRANSPORT_KEY, "Transport");

// IOMedia key constants
io_const!(IO_MEDIA_BSD_NAME_KEY, "BSD Name");
io_const!(IO_MEDIA_WHOLE_KEY, "Whole");
io_const!(IO_MEDIA_LEAF_KEY, "Leaf");
io_const!(IO_MEDIA_SIZE_KEY, "Size");

// IOPM level constants — stored as u32 values
static IOPM_ASSERTION_LEVEL_ON: u32 = 255;
static IOPM_ASSERTION_LEVEL_OFF: u32 = 0;

// IORegistry entry ID counter (synthetic unique IDs for entries)
static NEXT_ENTRY_ID: AtomicU64 = AtomicU64::new(0x100_0000_0001);
/// Maps service handle → assigned entry ID
static ENTRY_IDS: Mutex<Option<HashMap<u32, u64>>> = Mutex::new(None);

fn get_entry_id(handle: u32) -> u64 {
    let mut guard = ENTRY_IDS.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    *map.entry(handle)
        .or_insert_with(|| NEXT_ENTRY_ID.fetch_add(1, Ordering::Relaxed))
}

/// Maps absolute IOService path → handle for IORegistryEntryFromPath
static PATH_REGISTRY: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);

fn register_path(path: &str, handle: u32) {
    let mut guard = PATH_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(path.to_string(), handle);
}

fn lookup_by_path(path: &str) -> Option<u32> {
    PATH_REGISTRY.lock().unwrap().as_ref()?.get(path).copied()
}

// ── Install ─────────────────────────────────────────────────────────────────

impl IOKitStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── Service matching & lifecycle ────────────────────────
        register_shims!(
            table,
            (
                "_IOServiceGetMatchingService",
                shim_io_service_get_matching_service as *const () as u64,
                "IOServiceGetMatchingService → synthetic service from registry"
            ),
            (
                "_IOServiceGetMatchingServices",
                shim_io_service_get_matching_services as *const () as u64,
                "IOServiceGetMatchingServices → iterator over matching services"
            ),
            (
                "_IOServiceMatching",
                shim_io_service_matching as *const () as u64,
                "IOServiceMatching → tracked matching dict"
            ),
            (
                "_IOServiceNameMatching",
                shim_io_service_matching as *const () as u64,
                "IOServiceNameMatching → tracked matching dict"
            ),
            (
                "_IOServiceOpen",
                shim_io_service_open as *const () as u64,
                "IOServiceOpen → kIOReturnSuccess, synthetic port"
            ),
            (
                "_IOServiceClose",
                shim_io_return_success as *const () as u64,
                "IOServiceClose → kIOReturnSuccess"
            ),
            (
                "_IOServiceAddNotification",
                shim_io_return_success as *const () as u64,
                "IOServiceAddNotification → kIOReturnSuccess"
            ),
            (
                "_IOServiceAddMatchingNotification",
                shim_io_return_success as *const () as u64,
                "IOServiceAddMatchingNotification → kIOReturnSuccess"
            ),
            (
                "_IOObjectRelease",
                shim_io_return_success as *const () as u64,
                "IOObjectRelease → kIOReturnSuccess"
            ),
            (
                "_IOObjectRetain",
                shim_io_return_success as *const () as u64,
                "IOObjectRetain → kIOReturnSuccess"
            ),
            (
                "_IOObjectGetClass",
                shim_io_object_get_class as *const () as u64,
                "IOObjectGetClass → class name from registry"
            ),
            (
                "_IOObjectConformsTo",
                shim_io_object_conforms_to as *const () as u64,
                "IOObjectConformsTo → 1 if class matches"
            ),
            (
                "_IOIteratorNext",
                shim_io_iterator_next as *const () as u64,
                "IOIteratorNext → next service or 0"
            ),
            (
                "_IOIteratorReset",
                shim_io_iterator_reset as *const () as u64,
                "IOIteratorReset → rewind"
            ),
            (
                "_IOIteratorIsValid",
                shim_io_one_stub as *const () as u64,
                "IOIteratorIsValid → 1 (always valid)"
            )
        );

        // ── Master / main port ──────────────────────────────────
        register_shims!(
            table,
            (
                "_IOMasterPort",
                shim_io_master_port as *const () as u64,
                "IOMasterPort → kIOReturnSuccess, port=0"
            ),
            (
                "_IOMainPort",
                shim_io_master_port as *const () as u64,
                "IOMainPort → kIOReturnSuccess, port=0"
            )
        );

        // ── Registry entry ──────────────────────────────────────
        register_shims!(
            table,
            (
                "_IORegistryEntryCreateCFProperty",
                shim_io_registry_entry_create_cf_property as *const () as u64,
                "IORegistryEntryCreateCFProperty → sysfs-backed value"
            ),
            (
                "_IORegistryEntryGetName",
                shim_io_registry_entry_get_name as *const () as u64,
                "IORegistryEntryGetName → class name"
            ),
            (
                "_IORegistryEntryGetPath",
                shim_io_registry_entry_get_path as *const () as u64,
                "IORegistryEntryGetPath → synthetic IOService path"
            ),
            (
                "_IORegistryEntryCreateCFProperties",
                shim_io_registry_entry_create_cf_properties as *const () as u64,
                "IORegistryEntryCreateCFProperties → sysfs-backed dict"
            ),
            (
                "_IORegistryEntrySearchCFProperty",
                shim_io_registry_entry_search_cf_property as *const () as u64,
                "IORegistryEntrySearchCFProperty → delegates to CreateCFProperty"
            ),
            (
                "_IORegistryEntryGetParentEntry",
                shim_io_registry_entry_get_parent as *const () as u64,
                "IORegistryEntryGetParentEntry → root or 0"
            ),
            (
                "_IORegistryEntryGetChildEntry",
                shim_io_return_not_found as *const () as u64,
                "IORegistryEntryGetChildEntry → kIOReturnNotFound"
            ),
            (
                "_IORegistryEntryGetChildIterator",
                shim_io_registry_entry_get_child_iterator as *const () as u64,
                "IORegistryEntryGetChildIterator → empty iterator"
            ),
            (
                "_IORegistryGetRootEntry",
                shim_io_registry_get_root_entry as *const () as u64,
                "IORegistryGetRootEntry → platform expert handle"
            )
        );

        // ── Connect / call ──────────────────────────────────────
        register_shims!(
            table,
            (
                "_IOConnectCallMethod",
                shim_io_connect_call_method as *const () as u64,
                "IOConnectCallMethod → kIOReturnUnsupported"
            ),
            (
                "_IOConnectCallStructMethod",
                shim_io_connect_call_struct_method as *const () as u64,
                "IOConnectCallStructMethod → kIOReturnUnsupported"
            ),
            (
                "_IOConnectCallScalarMethod",
                shim_io_connect_call_scalar_method as *const () as u64,
                "IOConnectCallScalarMethod → kIOReturnUnsupported"
            ),
            (
                "_IOConnectSetNotificationPort",
                shim_io_return_success as *const () as u64,
                "IOConnectSetNotificationPort → kIOReturnSuccess"
            ),
            (
                "_IOConnectMapMemory",
                shim_io_return_unsupported_stub as *const () as u64,
                "IOConnectMapMemory → kIOReturnUnsupported"
            ),
            (
                "_IOConnectUnmapMemory",
                shim_io_return_success as *const () as u64,
                "IOConnectUnmapMemory → kIOReturnSuccess"
            )
        );

        // ── Registry entry — extended ───────────────────────────
        register_shims!(
            table,
            (
                "_IORegistryEntryGetRegistryEntryID",
                shim_io_registry_entry_get_id as *const () as u64,
                "IORegistryEntryGetRegistryEntryID → unique 64-bit ID"
            ),
            (
                "_IORegistryEntryFromPath",
                shim_io_registry_entry_from_path as *const () as u64,
                "IORegistryEntryFromPath → lookup by path or 0"
            ),
            (
                "_IORegistryEntryGetLocationInPlane",
                shim_io_registry_entry_get_location as *const () as u64,
                "IORegistryEntryGetLocationInPlane → \"0\""
            ),
            (
                "_IORegistryEntrySetCFProperties",
                shim_io_return_success as *const () as u64,
                "IORegistryEntrySetCFProperties → kIOReturnSuccess"
            ),
            (
                "_IORegistryEntrySetCFProperty",
                shim_io_return_success as *const () as u64,
                "IORegistryEntrySetCFProperty → kIOReturnSuccess"
            ),
            (
                "_IORegistryEntryInPlane",
                shim_io_one_stub as *const () as u64,
                "IORegistryEntryInPlane → 1 (yes)"
            )
        );

        // ── BSD name matching ───────────────────────────────────
        register_shims!(
            table,
            (
                "_IOBSDNameMatching",
                shim_io_bsd_name_matching as *const () as u64,
                "IOBSDNameMatching → tracked matching dict"
            ),
            (
                "_IOServiceWaitQuiet",
                shim_io_return_success as *const () as u64,
                "IOServiceWaitQuiet → kIOReturnSuccess (instant)"
            ),
            (
                "_IOKitGetBusyState",
                shim_io_kit_get_busy_state as *const () as u64,
                "IOKitGetBusyState → 0 (not busy)"
            )
        );

        // ── Plugin interface ────────────────────────────────────
        register_shims!(
            table,
            (
                "_IOCreatePlugInInterfaceForService",
                shim_io_create_plugin_interface as *const () as u64,
                "IOCreatePlugInInterfaceForService → kIOReturnUnsupported"
            ),
            (
                "_IODestroyPlugInInterface",
                shim_io_return_success as *const () as u64,
                "IODestroyPlugInInterface → kIOReturnSuccess"
            )
        );

        // ── Serialization ───────────────────────────────────────
        register_shims!(
            table,
            (
                "_IOCFSerialize",
                shim_io_null_stub as *const () as u64,
                "IOCFSerialize → NULL (deferred)"
            ),
            (
                "_IOCFUnserialize",
                shim_io_null_stub as *const () as u64,
                "IOCFUnserialize → NULL (deferred)"
            )
        );

        // ── Display info ────────────────────────────────────────
        register_shims!(
            table,
            (
                "_IODisplayCreateInfoDictionary",
                shim_io_display_create_info_dict as *const () as u64,
                "IODisplayCreateInfoDictionary → display props from sysfs"
            ),
            (
                "_IODisplayGetParameters",
                shim_io_return_unsupported_stub as *const () as u64,
                "IODisplayGetParameters → kIOReturnUnsupported"
            ),
            (
                "_IODisplaySetParameters",
                shim_io_return_unsupported_stub as *const () as u64,
                "IODisplaySetParameters → kIOReturnUnsupported"
            ),
            (
                "_IODisplayGetFloatParameter",
                shim_io_return_unsupported_stub as *const () as u64,
                "IODisplayGetFloatParameter → kIOReturnUnsupported"
            )
        );

        // ── IOSurface stubs ─────────────────────────────────────
        register_shims!(
            table,
            (
                "_IOSurfaceCreate",
                shim_io_null_stub as *const () as u64,
                "IOSurfaceCreate → NULL"
            ),
            (
                "_IOSurfaceLookup",
                shim_io_null_stub as *const () as u64,
                "IOSurfaceLookup → NULL"
            ),
            (
                "_IOSurfaceGetBaseAddress",
                shim_io_null_stub as *const () as u64,
                "IOSurfaceGetBaseAddress → NULL"
            ),
            (
                "_IOSurfaceGetWidth",
                shim_io_zero_u32_stub as *const () as u64,
                "IOSurfaceGetWidth → 0"
            ),
            (
                "_IOSurfaceGetHeight",
                shim_io_zero_u32_stub as *const () as u64,
                "IOSurfaceGetHeight → 0"
            ),
            (
                "_IOSurfaceGetBytesPerRow",
                shim_io_zero_u32_stub as *const () as u64,
                "IOSurfaceGetBytesPerRow → 0"
            ),
            (
                "_IOSurfaceLock",
                shim_io_return_success as *const () as u64,
                "IOSurfaceLock → kIOReturnSuccess"
            ),
            (
                "_IOSurfaceUnlock",
                shim_io_return_success as *const () as u64,
                "IOSurfaceUnlock → kIOReturnSuccess"
            )
        );

        // ── Power management — extended ─────────────────────────
        register_shims!(
            table,
            (
                "_IOPMAssertionCopyProperties",
                shim_iopm_assertion_copy_properties as *const () as u64,
                "IOPMAssertionCopyProperties → dict of assertion props"
            ),
            (
                "_IOPMConnectionCreate",
                shim_io_return_success as *const () as u64,
                "IOPMConnectionCreate → kIOReturnSuccess"
            ),
            (
                "_IOPMConnectionRelease",
                shim_io_void_stub as *const () as u64,
                "IOPMConnectionRelease → void"
            ),
            (
                "_IOPMConnectionSetNotification",
                shim_io_return_success as *const () as u64,
                "IOPMConnectionSetNotification → kIOReturnSuccess"
            ),
            (
                "_IOPMConnectionScheduleWithRunLoop",
                shim_io_return_success as *const () as u64,
                "IOPMConnectionScheduleWithRunLoop → kIOReturnSuccess"
            ),
            (
                "_IOPMSleepSystem",
                shim_io_return_unsupported_stub as *const () as u64,
                "IOPMSleepSystem → kIOReturnUnsupported"
            ),
            (
                "_IOPMSetAggressiveness",
                shim_io_return_success as *const () as u64,
                "IOPMSetAggressiveness → kIOReturnSuccess"
            ),
            (
                "_IOPMGetAggressiveness",
                shim_io_return_success as *const () as u64,
                "IOPMGetAggressiveness → kIOReturnSuccess"
            )
        );

        // ── IOPSNotification ────────────────────────────────────
        register_shims!(
            table,
            (
                "_IOPSNotificationCreateRunLoopSource",
                shim_io_null_stub as *const () as u64,
                "IOPSNotificationCreateRunLoopSource → NULL"
            ),
            (
                "_IOPSGetTimeRemainingEstimate",
                shim_iops_get_time_remaining as *const () as u64,
                "IOPSGetTimeRemainingEstimate → seconds or sentinel"
            )
        );

        // ── Notification port ───────────────────────────────────
        register_shims!(
            table,
            (
                "_IONotificationPortCreate",
                shim_io_notification_port_create as *const () as u64,
                "IONotificationPortCreate → synthetic port"
            ),
            (
                "_IONotificationPortDestroy",
                shim_io_void_stub as *const () as u64,
                "IONotificationPortDestroy → void"
            ),
            (
                "_IONotificationPortGetRunLoopSource",
                shim_io_null_stub as *const () as u64,
                "IONotificationPortGetRunLoopSource → NULL"
            ),
            (
                "_IONotificationPortGetMachPort",
                shim_io_zero_u32_stub as *const () as u64,
                "IONotificationPortGetMachPort → 0"
            ),
            (
                "_IONotificationPortSetDispatchQueue",
                shim_io_void_stub as *const () as u64,
                "IONotificationPortSetDispatchQueue → void"
            )
        );

        // ── Power management assertions ─────────────────────────
        register_shims!(
            table,
            (
                "_IOPMAssertionCreateWithName",
                shim_iopm_assertion_create as *const () as u64,
                "IOPMAssertionCreateWithName → tracked assertion"
            ),
            (
                "_IOPMAssertionCreateWithProperties",
                shim_iopm_assertion_create_with_props as *const () as u64,
                "IOPMAssertionCreateWithProperties → tracked assertion"
            ),
            (
                "_IOPMAssertionRelease",
                shim_iopm_assertion_release as *const () as u64,
                "IOPMAssertionRelease → release tracked assertion"
            ),
            (
                "_IOPMAssertionSetProperty",
                shim_io_return_success as *const () as u64,
                "IOPMAssertionSetProperty → kIOReturnSuccess"
            ),
            (
                "_IOPMAssertionDeclareUserActivity",
                shim_iopm_assertion_create as *const () as u64,
                "IOPMAssertionDeclareUserActivity → tracked assertion"
            ),
            (
                "_IOPMCopyFeatureDefault",
                shim_io_null_stub as *const () as u64,
                "IOPMCopyFeatureDefault → NULL"
            ),
            (
                "_IOPMCopyBatteryInfo",
                shim_iopm_copy_battery_info as *const () as u64,
                "IOPMCopyBatteryInfo → sysfs battery or unsupported"
            )
        );

        // ── Power source info ───────────────────────────────────
        register_shims!(
            table,
            (
                "_IOPSCopyPowerSourcesInfo",
                shim_iops_copy_power_sources_info as *const () as u64,
                "IOPSCopyPowerSourcesInfo → sysfs blob"
            ),
            (
                "_IOPSCopyPowerSourcesList",
                shim_iops_copy_power_sources_list as *const () as u64,
                "IOPSCopyPowerSourcesList → array of power sources"
            ),
            (
                "_IOPSGetPowerSourceDescription",
                shim_iops_get_power_source_description as *const () as u64,
                "IOPSGetPowerSourceDescription → sysfs-backed dict"
            ),
            (
                "_IOPSGetProvidingPowerSourceType",
                shim_iops_get_providing_type as *const () as u64,
                "IOPSGetProvidingPowerSourceType → AC/Battery string"
            )
        );

        // ── HID device & manager ────────────────────────────────
        register_shims!(
            table,
            (
                "_IOHIDManagerCreate",
                shim_io_null_stub as *const () as u64,
                "IOHIDManagerCreate → NULL"
            ),
            (
                "_IOHIDManagerOpen",
                shim_io_return_success as *const () as u64,
                "IOHIDManagerOpen → kIOReturnSuccess"
            ),
            (
                "_IOHIDManagerClose",
                shim_io_return_success as *const () as u64,
                "IOHIDManagerClose → kIOReturnSuccess"
            ),
            (
                "_IOHIDManagerSetDeviceMatching",
                shim_io_void_stub as *const () as u64,
                "IOHIDManagerSetDeviceMatching → void"
            ),
            (
                "_IOHIDManagerCopyDevices",
                shim_io_null_stub as *const () as u64,
                "IOHIDManagerCopyDevices → NULL (no HID devices)"
            ),
            (
                "_IOHIDManagerScheduleWithRunLoop",
                shim_io_void_stub as *const () as u64,
                "IOHIDManagerScheduleWithRunLoop → void"
            ),
            (
                "_IOHIDManagerRegisterDeviceMatchingCallback",
                shim_io_void_stub as *const () as u64,
                "IOHIDManagerRegisterDeviceMatchingCallback → void"
            ),
            (
                "_IOHIDManagerRegisterDeviceRemovalCallback",
                shim_io_void_stub as *const () as u64,
                "IOHIDManagerRegisterDeviceRemovalCallback → void"
            ),
            (
                "_IOHIDDeviceCreate",
                shim_io_null_stub as *const () as u64,
                "IOHIDDeviceCreate → NULL"
            ),
            (
                "_IOHIDDeviceOpen",
                shim_io_return_success as *const () as u64,
                "IOHIDDeviceOpen → kIOReturnSuccess"
            ),
            (
                "_IOHIDDeviceClose",
                shim_io_return_success as *const () as u64,
                "IOHIDDeviceClose → kIOReturnSuccess"
            ),
            (
                "_IOHIDDeviceGetProperty",
                shim_io_null_stub as *const () as u64,
                "IOHIDDeviceGetProperty → NULL"
            ),
            (
                "_IOHIDDeviceCopyMatchingElements",
                shim_io_null_stub as *const () as u64,
                "IOHIDDeviceCopyMatchingElements → NULL"
            )
        );

        // ── Data symbols ────────────────────────────────────────
        register_shims!(
            table,
            (
                "_kIOMasterPortDefault",
                &K_IO_MASTER_PORT_DEFAULT as *const u32 as u64,
                "kIOMasterPortDefault data symbol (u32=0)"
            ),
            (
                "_kIOMainPortDefault",
                &K_IO_MASTER_PORT_DEFAULT as *const u32 as u64,
                "kIOMainPortDefault data symbol (u32=0)"
            ),
            (
                "_kIOPMAssertionTypeNoIdleSleep",
                &IOPM_NO_IDLE_SLEEP as *const _ as u64,
                "kIOPMAssertionTypeNoIdleSleep data symbol"
            ),
            (
                "_kIOPMAssertionTypeNoDisplaySleep",
                &IOPM_NO_DISPLAY_SLEEP as *const _ as u64,
                "kIOPMAssertionTypeNoDisplaySleep data symbol"
            ),
            (
                "_kIOPMAssertionTypePreventUserIdleSystemSleep",
                &IOPM_PREVENT_USER_IDLE_SYSTEM as *const _ as u64,
                "kIOPMAssertionTypePreventUserIdleSystemSleep data symbol"
            ),
            (
                "_kIOPMAssertionTypePreventUserIdleDisplaySleep",
                &IOPM_PREVENT_USER_IDLE_DISPLAY as *const _ as u64,
                "kIOPMAssertionTypePreventUserIdleDisplaySleep data symbol"
            ),
            (
                "_kIOPMAssertionTypePreventSystemSleep",
                &IOPM_PREVENT_SYSTEM_SLEEP as *const _ as u64,
                "kIOPMAssertionTypePreventSystemSleep data symbol"
            ),
            (
                "_kIOPMAssertionTypeExternalMedia",
                &IOPM_EXTERNAL_MEDIA as *const _ as u64,
                "kIOPMAssertionTypeExternalMedia data symbol"
            ),
            (
                "_kIOPMAssertionTypeNetworkClientActive",
                &IOPM_NETWORK_CLIENT as *const _ as u64,
                "kIOPMAssertionTypeNetworkClientActive data symbol"
            ),
            (
                "_kIOServicePlane",
                &IO_SERVICE_PLANE as *const _ as u64,
                "kIOServicePlane data symbol"
            ),
            (
                "_kIOPowerPlane",
                &IO_POWER_PLANE as *const _ as u64,
                "kIOPowerPlane data symbol"
            ),
            (
                "_kIODeviceTreePlane",
                &IO_DEVICE_TREE_PLANE as *const _ as u64,
                "kIODeviceTreePlane data symbol"
            ),
            (
                "_kIOUSBPlane",
                &IO_USB_PLANE as *const _ as u64,
                "kIOUSBPlane data symbol"
            ),
            (
                "_kIOFireWirePlane",
                &IO_FIREWIRE_PLANE as *const _ as u64,
                "kIOFireWirePlane data symbol"
            ),
            (
                "_kIOPMAssertionLevelOn",
                &IOPM_ASSERTION_LEVEL_ON as *const u32 as u64,
                "kIOPMAssertionLevelOn data symbol (u32=255)"
            ),
            (
                "_kIOPMAssertionLevelOff",
                &IOPM_ASSERTION_LEVEL_OFF as *const u32 as u64,
                "kIOPMAssertionLevelOff data symbol (u32=0)"
            ),
            (
                "_kIOUSBDeviceClassName",
                &IO_USB_DEVICE_CLASS_NAME as *const _ as u64,
                "kIOUSBDeviceClassName data symbol"
            ),
            (
                "_kIOUSBInterfaceClassName",
                &IO_USB_INTERFACE_CLASS_NAME as *const _ as u64,
                "kIOUSBInterfaceClassName data symbol"
            ),
            (
                "_kIOHIDDeviceKey",
                &IO_HID_DEVICE_KEY as *const _ as u64,
                "kIOHIDDeviceKey data symbol"
            ),
            (
                "_kIOHIDProductKey",
                &IO_HID_PRODUCT_KEY as *const _ as u64,
                "kIOHIDProductKey data symbol"
            ),
            (
                "_kIOHIDVendorIDKey",
                &IO_HID_VENDOR_ID_KEY as *const _ as u64,
                "kIOHIDVendorIDKey data symbol"
            ),
            (
                "_kIOHIDProductIDKey",
                &IO_HID_PRODUCT_ID_KEY as *const _ as u64,
                "kIOHIDProductIDKey data symbol"
            ),
            (
                "_kIOHIDTransportKey",
                &IO_HID_TRANSPORT_KEY as *const _ as u64,
                "kIOHIDTransportKey data symbol"
            ),
            (
                "_kIOMediaBSDNameKey",
                &IO_MEDIA_BSD_NAME_KEY as *const _ as u64,
                "kIOMediaBSDNameKey data symbol"
            ),
            (
                "_kIOMediaWholeKey",
                &IO_MEDIA_WHOLE_KEY as *const _ as u64,
                "kIOMediaWholeKey data symbol"
            ),
            (
                "_kIOMediaLeafKey",
                &IO_MEDIA_LEAF_KEY as *const _ as u64,
                "kIOMediaLeafKey data symbol"
            ),
            (
                "_kIOMediaSizeKey",
                &IO_MEDIA_SIZE_KEY as *const _ as u64,
                "kIOMediaSizeKey data symbol"
            )
        );

        log::debug!("IOKitStubs: registered 130+ IOKit shims + 28 data symbols");
    }
}

// ── Service matching ────────────────────────────────────────────────────────

/// `IOServiceMatching(name) → CFMutableDictionaryRef`
///
/// Creates a tracked matching dictionary keyed by class name.
/// Returns a synthetic handle that `IOServiceGetMatchingService` can look up.
pub extern "C" fn shim_io_service_matching(name: *const u8) -> u64 {
    if name.is_null() {
        return 0;
    }
    let class_name = unsafe { CStr::from_ptr(name as *const _) }
        .to_string_lossy()
        .into_owned();
    log::debug!("IOServiceMatching(\"{}\")", class_name);
    track_matching_dict(&class_name)
}

/// `IOServiceGetMatchingService(port, matching) → io_service_t`
///
/// Looks up the class from the tracked matching dict, then finds the first
/// matching synthetic service in the registry.
pub extern "C" fn shim_io_service_get_matching_service(_master_port: u32, matching: u64) -> u32 {
    let Some(class_name) = lookup_matching_class(matching) else {
        return 0;
    };
    log::debug!("IOServiceGetMatchingService(\"{}\")", class_name);
    find_service_by_class(&class_name).unwrap_or(0)
}

/// `IOServiceGetMatchingServices(port, matching, &iterator) → kern_return_t`
///
/// Populates an iterator with all matching services.
pub extern "C" fn shim_io_service_get_matching_services(
    _master_port: u32,
    matching: u64,
    existing: *mut u32,
) -> i32 {
    let Some(class_name) = lookup_matching_class(matching) else {
        if !existing.is_null() {
            unsafe {
                *existing = 0;
            }
        }
        return K_IO_RETURN_SUCCESS;
    };

    let services = ensure_registry();
    let matching_handles: Vec<u32> = services
        .iter()
        .filter(|s| s.class_name == class_name)
        .map(|s| s.handle)
        .collect();

    let iter_handle = create_iterator(matching_handles);
    if !existing.is_null() {
        unsafe {
            *existing = iter_handle;
        }
    }
    K_IO_RETURN_SUCCESS
}

/// `IOServiceOpen` → kIOReturnSuccess with synthetic connect port.
pub extern "C" fn shim_io_service_open(
    _service: u32,
    _owning_task: u32,
    _type_: u32,
    connect: *mut u32,
) -> i32 {
    if !connect.is_null() {
        unsafe {
            *connect = alloc_handle();
        }
    }
    K_IO_RETURN_SUCCESS
}

// ── Master / main port ──────────────────────────────────────────────────────

/// `IOMasterPort(bootstrap, &port) → kern_return_t`
pub extern "C" fn shim_io_master_port(_bootstrap: u32, port: *mut u32) -> i32 {
    if !port.is_null() {
        unsafe {
            *port = 0;
        } // MACH_PORT_NULL — same as kIOMasterPortDefault
    }
    K_IO_RETURN_SUCCESS
}

// ── Generic stubs ───────────────────────────────────────────────────────────

/// Generic stub returning `kIOReturnSuccess`.
pub extern "C" fn shim_io_return_success(_a: u64) -> i32 {
    K_IO_RETURN_SUCCESS
}

/// Generic stub returning `kIOReturnNotFound`.
pub extern "C" fn shim_io_return_not_found(_a: u64, _b: u64, _c: *mut u32) -> i32 {
    K_IO_RETURN_NOT_FOUND
}

/// Generic stub returning `kIOReturnUnsupported` (multi-arg).
pub extern "C" fn shim_io_return_unsupported_stub(_a: u64, _b: u64, _c: u64, _d: u64) -> i32 {
    K_IO_RETURN_UNSUPPORTED
}

/// Generic stub returning NULL.
pub extern "C" fn shim_io_null_stub(_a: u64) -> u64 {
    0 // NULL
}

/// Generic stub returning 1 (true / valid).
pub extern "C" fn shim_io_one_stub(_a: u64) -> u32 {
    1
}

/// Generic stub returning 0 as u32.
pub extern "C" fn shim_io_zero_u32_stub(_a: u64) -> u32 {
    0
}

/// Generic void stub (e.g. IOIteratorReset, IONotificationPortDestroy).
pub extern "C" fn shim_io_void_stub(_a: u64) {}

// ── Object / iterator ───────────────────────────────────────────────────────

/// `IOObjectGetClass(obj, class_buf) → kern_return_t`
///
/// Writes the class name of a synthetic service into the caller's buffer.
pub extern "C" fn shim_io_object_get_class(obj: u32, class_buf: *mut u8) -> i32 {
    if class_buf.is_null() {
        return K_IO_RETURN_UNSUPPORTED;
    }
    if let Some(service) = find_service_by_handle(obj) {
        let name = service.class_name.as_bytes();
        let len = name.len().min(127); // IOKit max class name is 128 chars
        unsafe {
            std::ptr::copy_nonoverlapping(name.as_ptr(), class_buf, len);
            *class_buf.add(len) = 0;
        }
        K_IO_RETURN_SUCCESS
    } else {
        // Unknown handle — write "IOObject"
        let fallback = b"IOObject\0";
        unsafe {
            std::ptr::copy_nonoverlapping(fallback.as_ptr(), class_buf, fallback.len());
        }
        K_IO_RETURN_SUCCESS
    }
}

/// `IOObjectConformsTo(obj, className) → boolean_t`
pub extern "C" fn shim_io_object_conforms_to(obj: u32, class_name: *const u8) -> u32 {
    if class_name.is_null() {
        return 0;
    }
    let target = unsafe { CStr::from_ptr(class_name as *const _) }.to_string_lossy();
    if let Some(service) = find_service_by_handle(obj) {
        if service.class_name == target.as_ref() {
            return 1;
        }
    }
    0
}

/// `IOIteratorNext(iterator) → io_object_t`
///
/// Returns the next service handle from the iterator, or 0 when exhausted.
pub extern "C" fn shim_io_iterator_next(iterator: u32) -> u32 {
    let mut guard = ITERATORS.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some((services, idx)) = map.get_mut(&iterator) {
            if *idx < services.len() {
                let handle = services[*idx];
                *idx += 1;
                return handle;
            }
        }
    }
    0 // IO_OBJECT_NULL — end of iteration
}

/// `IOIteratorReset(iterator)`
pub extern "C" fn shim_io_iterator_reset(iterator: u32) {
    let mut guard = ITERATORS.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some((_services, idx)) = map.get_mut(&iterator) {
            *idx = 0;
        }
    }
}

// ── Notification port ───────────────────────────────────────────────────────

/// `IONotificationPortCreate(masterPort) → IONotificationPortRef`
pub extern "C" fn shim_io_notification_port_create(_master_port: u32) -> u64 {
    alloc_handle() as u64
}

// ── Registry ────────────────────────────────────────────────────────────────

/// Extract a C string from a CFStringRef/key pointer for property lookup.
fn extract_key_string(key: u64) -> Option<String> {
    if key == 0 {
        return None;
    }
    let p = key as *const u8;
    unsafe {
        // Read first 8 bytes using unaligned read (pointer may not be 8-aligned)
        let first_word = std::ptr::read_unaligned(p as *const u64);
        // Check if first 8 bytes look like a small ISA (0x1, 0x2, 0x3) → CFString object
        if first_word <= 0x10 && first_word > 0 {
            // This is a CFString-like object; read length at +8, chars_ptr at +16
            let len = std::ptr::read_unaligned(p.add(8) as *const u64) as usize;
            let chars_ptr = std::ptr::read_unaligned(p.add(16) as *const u64) as *const u8;
            if len > 0 && len < 1024 && !chars_ptr.is_null() {
                let slice = std::slice::from_raw_parts(chars_ptr, len);
                return std::str::from_utf8(slice).ok().map(|s| s.to_string());
            }
        }
        // Try as raw C string
        let mut len = 0usize;
        while len < 256 {
            let byte = *p.add(len);
            if byte == 0 {
                break;
            }
            if byte > 127 {
                return None;
            }
            len += 1;
        }
        if len > 0 && len < 256 {
            let slice = std::slice::from_raw_parts(p, len);
            std::str::from_utf8(slice).ok().map(|s| s.to_string())
        } else {
            None
        }
    }
}

/// Look up a single property for a synthetic service by key name.
fn lookup_service_property(service_handle: u32, key: &str) -> u64 {
    let Some(service) = find_service_by_handle(service_handle) else {
        return 0;
    };

    match service.class_name {
        "IOPlatformExpertDevice" | "AppleACPIPlatformExpert" => match key {
            "IOPlatformSerialNumber" => {
                let serial = read_sysfs("/sys/class/dmi/id/product_serial")
                    .unwrap_or_else(|| "BURDETTIE0001".to_string());
                alloc_cfstring(&serial)
            }
            "IOPlatformUUID" => {
                let uuid = read_sysfs("/sys/class/dmi/id/product_uuid")
                    .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string());
                alloc_cfstring(&uuid)
            }
            "model" => {
                let model = read_sysfs("/sys/class/dmi/id/product_name")
                    .unwrap_or_else(|| "BurdettieOS Virtual Platform".to_string());
                alloc_cfstring(&model)
            }
            "board-id" => {
                let board = read_sysfs("/sys/class/dmi/id/board_name")
                    .unwrap_or_else(|| "BurdettieOS".to_string());
                alloc_cfstring(&board)
            }
            "manufacturer" => {
                let mfg = read_sysfs("/sys/class/dmi/id/sys_vendor")
                    .unwrap_or_else(|| "BurdettieOS".to_string());
                alloc_cfstring(&mfg)
            }
            _ => 0,
        },
        "IOPMPowerSource" | "AppleSmartBattery" => {
            #[cfg(target_os = "linux")]
            if let Some(bat_path) = first_battery_path() {
                return match key {
                    "CurrentCapacity" | "Current Capacity" => {
                        read_sysfs_i64(&format!("{}/energy_now", bat_path))
                            .or_else(|| read_sysfs_i64(&format!("{}/charge_now", bat_path)))
                            .map(alloc_cfnumber_i64)
                            .unwrap_or(0)
                    }
                    "MaxCapacity" | "Max Capacity" => {
                        read_sysfs_i64(&format!("{}/energy_full", bat_path))
                            .or_else(|| read_sysfs_i64(&format!("{}/charge_full", bat_path)))
                            .map(alloc_cfnumber_i64)
                            .unwrap_or(0)
                    }
                    "IsCharging" | "Is Charging" => {
                        let status =
                            read_sysfs(&format!("{}/status", bat_path)).unwrap_or_default();
                        alloc_cfbool(status == "Charging")
                    }
                    "ExternalConnected" => {
                        let ac = read_sysfs("/sys/class/power_supply/AC/online")
                            .unwrap_or_else(|| "0".to_string());
                        alloc_cfbool(ac.trim() == "1")
                    }
                    "DesignCapacity" => read_sysfs_i64(&format!("{}/energy_full_design", bat_path))
                        .or_else(|| read_sysfs_i64(&format!("{}/charge_full_design", bat_path)))
                        .map(alloc_cfnumber_i64)
                        .unwrap_or(0),
                    "CycleCount" => read_sysfs_i64(&format!("{}/cycle_count", bat_path))
                        .map(alloc_cfnumber_i64)
                        .unwrap_or(0),
                    "Temperature" => read_sysfs_i64(&format!("{}/temp", bat_path))
                        .map(alloc_cfnumber_i64)
                        .unwrap_or(0),
                    _ => 0,
                };
            }
            0
        }
        "IOBlockStorageDevice" | "IOMedia" => match key {
            "BSD Name" | "BSD name" => {
                #[cfg(target_os = "linux")]
                if let Ok(entries) = std::fs::read_dir("/sys/block") {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name = name.to_string_lossy();
                        if name.starts_with("sd")
                            || name.starts_with("vd")
                            || name.starts_with("nvme")
                            || name.starts_with("mmcblk")
                        {
                            return alloc_cfstring(&name);
                        }
                    }
                }
                alloc_cfstring("disk0")
            }
            "Size" | "size" => {
                #[cfg(target_os = "linux")]
                if let Ok(entries) = std::fs::read_dir("/sys/block") {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name = name.to_string_lossy();
                        if name.starts_with("sd")
                            || name.starts_with("vd")
                            || name.starts_with("nvme")
                            || name.starts_with("mmcblk")
                        {
                            if let Some(size) = read_sysfs_i64(&format!("/sys/block/{}/size", name))
                            {
                                return alloc_cfnumber_i64(size * 512);
                            }
                        }
                    }
                }
                0
            }
            "Whole" => alloc_cfbool(true),
            "Leaf" => alloc_cfbool(false),
            "Content" => alloc_cfstring("GUID_partition_scheme"),
            _ => 0,
        },
        "IONetworkInterface" | "IOEthernetInterface" => {
            match key {
                "BSD Name" | "BSD name" => {
                    #[cfg(target_os = "linux")]
                    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
                        for entry in entries.flatten() {
                            let name = entry.file_name();
                            let name = name.to_string_lossy();
                            if name != "lo" {
                                return alloc_cfstring(&name);
                            }
                        }
                    }
                    alloc_cfstring("en0")
                }
                "IOMACAddress" => {
                    #[cfg(target_os = "linux")]
                    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
                        for entry in entries.flatten() {
                            let name = entry.file_name();
                            let name = name.to_string_lossy();
                            if name != "lo" {
                                if let Some(mac) =
                                    read_sysfs(&format!("/sys/class/net/{}/address", name))
                                {
                                    return alloc_cfstring(&mac);
                                }
                            }
                        }
                    }
                    0
                }
                "IOInterfaceType" => alloc_cfnumber_i64(6), // IFT_ETHER
                _ => 0,
            }
        }
        "AppleACPICPU" => {
            match key {
                "CPUCount" => {
                    #[cfg(target_os = "linux")]
                    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
                        let count = entries
                            .flatten()
                            .filter(|e| {
                                let n = e.file_name();
                                let s = n.to_string_lossy();
                                s.starts_with("cpu") && s[3..].parse::<u32>().is_ok()
                            })
                            .count();
                        return alloc_cfnumber_i64(count as i64);
                    }
                    0
                }
                "CPUFrequency" => {
                    read_sysfs_i64("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
                        .map(|f| alloc_cfnumber_i64(f * 1000)) // kHz → Hz
                        .unwrap_or(0)
                }
                "CPUArchitecture" => {
                    #[cfg(target_arch = "aarch64")]
                    return alloc_cfstring("arm64");
                    #[cfg(target_arch = "x86_64")]
                    return alloc_cfstring("x86_64");
                    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
                    alloc_cfstring("unknown")
                }
                _ => 0,
            }
        }
        _ => 0,
    }
}

/// `IORegistryEntryCreateCFProperty(entry, key, alloc, opts) → CFTypeRef`
pub extern "C" fn shim_io_registry_entry_create_cf_property(
    entry: u32,
    key: u64,
    _allocator: u64,
    _options: u32,
) -> u64 {
    let Some(key_str) = extract_key_string(key) else {
        return 0;
    };
    log::debug!(
        "IORegistryEntryCreateCFProperty({}, \"{}\")",
        entry,
        key_str
    );
    lookup_service_property(entry, &key_str)
}

/// `IORegistryEntrySearchCFProperty(entry, plane, key, alloc, opts) → CFTypeRef`
///
/// Searches the registry tree — we just delegate to CreateCFProperty since our
/// tree is flat.
pub extern "C" fn shim_io_registry_entry_search_cf_property(
    entry: u32,
    _plane: *const u8,
    key: u64,
    _allocator: u64,
    _options: u32,
) -> u64 {
    let Some(key_str) = extract_key_string(key) else {
        return 0;
    };
    log::debug!(
        "IORegistryEntrySearchCFProperty({}, \"{}\")",
        entry,
        key_str
    );

    // Try the entry itself first
    let result = lookup_service_property(entry, &key_str);
    if result != 0 {
        return result;
    }

    // Search all services in registry (tree search behavior)
    for service in ensure_registry() {
        let r = lookup_service_property(service.handle, &key_str);
        if r != 0 {
            return r;
        }
    }
    0
}

/// `IORegistryEntryGetName(entry, name_buf) → kern_return_t`
pub extern "C" fn shim_io_registry_entry_get_name(entry: u32, name_buf: *mut u8) -> i32 {
    if name_buf.is_null() {
        return K_IO_RETURN_UNSUPPORTED;
    }
    if let Some(service) = find_service_by_handle(entry) {
        let name = service.class_name.as_bytes();
        let len = name.len().min(127);
        unsafe {
            std::ptr::copy_nonoverlapping(name.as_ptr(), name_buf, len);
            *name_buf.add(len) = 0;
        }
        K_IO_RETURN_SUCCESS
    } else {
        let fallback = b"Unknown\0";
        unsafe {
            std::ptr::copy_nonoverlapping(fallback.as_ptr(), name_buf, fallback.len());
        }
        K_IO_RETURN_SUCCESS
    }
}

/// `IORegistryEntryGetPath(entry, plane, path_buf) → kern_return_t`
pub extern "C" fn shim_io_registry_entry_get_path(
    entry: u32,
    _plane: *const u8,
    path_buf: *mut u8,
) -> i32 {
    if path_buf.is_null() {
        return K_IO_RETURN_UNSUPPORTED;
    }
    let class_name = find_service_by_handle(entry)
        .map(|s| s.class_name)
        .unwrap_or("IOObject");
    let path = format!("IOService:/{}\0", class_name);
    let len = path.len().min(511);
    unsafe {
        std::ptr::copy_nonoverlapping(path.as_ptr(), path_buf, len);
    }
    K_IO_RETURN_SUCCESS
}

/// `IORegistryEntryCreateCFProperties(entry, &props, alloc, opts) → kern_return_t`
pub extern "C" fn shim_io_registry_entry_create_cf_properties(
    entry: u32,
    properties: *mut u64,
    _allocator: u64,
    _options: u32,
) -> i32 {
    if properties.is_null() {
        return K_IO_RETURN_UNSUPPORTED;
    }
    let Some(service) = find_service_by_handle(entry) else {
        unsafe {
            *properties = 0;
        }
        return K_IO_RETURN_NOT_FOUND;
    };

    log::debug!(
        "IORegistryEntryCreateCFProperties(\"{}\")",
        service.class_name
    );

    let dict = match service.class_name {
        "IOPlatformExpertDevice" | "AppleACPIPlatformExpert" => build_platform_expert_props(),
        "IOPMPowerSource" | "AppleSmartBattery" => build_power_source_dict(),
        "IOBlockStorageDevice" | "IOMedia" => {
            let d = alloc_mutable_dict();
            let handle = service.handle;
            // Populate with the same keys as single-property lookup
            for key in &["BSD Name", "Size", "Whole", "Leaf", "Content"] {
                let val = lookup_service_property(handle, key);
                if val != 0 {
                    dict_set(d, alloc_cfstring(key), val);
                }
            }
            d
        }
        "IONetworkInterface" | "IOEthernetInterface" => {
            let d = alloc_mutable_dict();
            let handle = service.handle;
            for key in &["BSD Name", "IOMACAddress", "IOInterfaceType"] {
                let val = lookup_service_property(handle, key);
                if val != 0 {
                    dict_set(d, alloc_cfstring(key), val);
                }
            }
            d
        }
        _ => {
            // Return an empty dict for unknown services
            alloc_mutable_dict()
        }
    };

    unsafe {
        *properties = dict;
    }
    K_IO_RETURN_SUCCESS
}

/// `IORegistryGetRootEntry(masterPort) → io_registry_entry_t`
pub extern "C" fn shim_io_registry_get_root_entry(_master_port: u32) -> u32 {
    // Return the platform expert handle (always first in registry)
    ensure_registry();
    find_service_by_class("AppleACPIPlatformExpert")
        .or_else(|| find_service_by_class("IOPlatformExpertDevice"))
        .unwrap_or(0)
}

/// `IORegistryEntryGetParentEntry(entry, plane, &parent) → kern_return_t`
pub extern "C" fn shim_io_registry_entry_get_parent(
    _entry: u32,
    _plane: *const u8,
    parent: *mut u32,
) -> i32 {
    if !parent.is_null() {
        // Return the root entry as the parent
        unsafe {
            *parent = shim_io_registry_get_root_entry(0);
        }
    }
    K_IO_RETURN_SUCCESS
}

/// `IORegistryEntryGetChildIterator(entry, plane, &iter) → kern_return_t`
pub extern "C" fn shim_io_registry_entry_get_child_iterator(
    _entry: u32,
    _plane: *const u8,
    iter: *mut u32,
) -> i32 {
    if !iter.is_null() {
        // Empty iterator
        unsafe {
            *iter = create_iterator(vec![]);
        }
    }
    K_IO_RETURN_SUCCESS
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

/// `IOPMAssertionCreateWithName(type, level, name, &id) → kern_return_t`
///
/// Tracks the assertion with a unique ID.
pub extern "C" fn shim_iopm_assertion_create(
    assertion_type: u64,
    _assertion_level: i32,
    assertion_name: u64,
    assertion_id: *mut u32,
) -> i32 {
    let id = NEXT_ASSERTION_ID.fetch_add(1, Ordering::Relaxed);

    let type_str = extract_key_string(assertion_type).unwrap_or_else(|| "unknown".to_string());
    let name_str = extract_key_string(assertion_name).unwrap_or_else(|| "unnamed".to_string());

    log::debug!(
        "IOPMAssertionCreate(type=\"{}\", name=\"{}\") → id={}",
        type_str,
        name_str,
        id
    );

    {
        let mut guard = ACTIVE_ASSERTIONS.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(id, (type_str, name_str));
    }

    if !assertion_id.is_null() {
        unsafe {
            *assertion_id = id;
        }
    }
    K_IO_RETURN_SUCCESS
}

/// `IOPMAssertionCreateWithProperties(props, &id) → kern_return_t`
pub extern "C" fn shim_iopm_assertion_create_with_props(
    _props: u64,
    assertion_id: *mut u32,
) -> i32 {
    let id = NEXT_ASSERTION_ID.fetch_add(1, Ordering::Relaxed);

    log::debug!("IOPMAssertionCreateWithProperties → id={}", id);

    {
        let mut guard = ACTIVE_ASSERTIONS.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(id, ("properties".to_string(), "unnamed".to_string()));
    }

    if !assertion_id.is_null() {
        unsafe {
            *assertion_id = id;
        }
    }
    K_IO_RETURN_SUCCESS
}

/// `IOPMAssertionRelease(id) → kern_return_t`
pub extern "C" fn shim_iopm_assertion_release(assertion_id: u32) -> i32 {
    log::debug!("IOPMAssertionRelease(id={})", assertion_id);
    let mut guard = ACTIVE_ASSERTIONS.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(&assertion_id);
    }
    K_IO_RETURN_SUCCESS
}

/// `IOPMCopyBatteryInfo` → kIOReturnUnsupported (no battery representation).
pub extern "C" fn shim_iopm_copy_battery_info(_master_port: u32, info: *mut u64) -> i32 {
    // If battery exists on Linux, return a dict
    if has_sysfs_battery() {
        let dict = build_power_source_dict();
        if dict != 0 && !info.is_null() {
            unsafe {
                *info = dict;
            }
            return K_IO_RETURN_SUCCESS;
        }
    }
    K_IO_RETURN_UNSUPPORTED
}

// ── Power source info (IOPSCopy*) ───────────────────────────────────────────

/// `IOPSCopyPowerSourcesInfo() → CFTypeRef` (opaque info blob)
pub extern "C" fn shim_iops_copy_power_sources_info() -> u64 {
    // Return a synthetic handle that IOPSCopyPowerSourcesList can use
    alloc_handle() as u64
}

/// `IOPSCopyPowerSourcesList(info) → CFArrayRef`
pub extern "C" fn shim_iops_copy_power_sources_list(_info: u64) -> u64 {
    // Return a 1-element array containing a power source handle
    let source_handle = alloc_handle() as u64;
    let array = unsafe { libc::calloc(1, 24) } as u64;
    if array == 0 {
        return 0;
    }
    let items = unsafe { libc::calloc(1, 8) } as *mut u64;
    if items.is_null() {
        return array;
    }
    unsafe {
        *items = source_handle;
        *((array as usize + 8) as *mut u64) = 1u64; // count
        *((array as usize + 16) as *mut u64) = items as u64;
    }
    array
}

/// `IOPSGetPowerSourceDescription(info, source) → CFDictionaryRef`
pub extern "C" fn shim_iops_get_power_source_description(_info: u64, _source: u64) -> u64 {
    build_power_source_dict()
}

/// `IOPSGetProvidingPowerSourceType(snapshot) → CFStringRef`
pub extern "C" fn shim_iops_get_providing_type(_snapshot: u64) -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Some(online) = read_sysfs("/sys/class/power_supply/AC/online") {
            if online.trim() == "1" {
                return alloc_cfstring("AC Power");
            }
        }
        if has_sysfs_battery() {
            return alloc_cfstring("Battery Power");
        }
    }
    alloc_cfstring("AC Power")
}

// ── Registry entry — extended ───────────────────────────────────────────────

/// `IORegistryEntryGetRegistryEntryID(entry, &entryID) → kern_return_t`
///
/// Returns a unique 64-bit ID for each registry entry. The ID is stable
/// across calls for the same handle.
pub extern "C" fn shim_io_registry_entry_get_id(entry: u32, entry_id: *mut u64) -> i32 {
    if entry_id.is_null() {
        return K_IO_RETURN_UNSUPPORTED;
    }
    let id = get_entry_id(entry);
    unsafe {
        *entry_id = id;
    }
    K_IO_RETURN_SUCCESS
}

/// `IORegistryEntryFromPath(masterPort, path) → io_registry_entry_t`
///
/// Looks up a registry entry by absolute path (e.g. "IOService:/IOPlatformExpertDevice").
pub extern "C" fn shim_io_registry_entry_from_path(_master_port: u32, path: *const u8) -> u32 {
    if path.is_null() {
        return 0;
    }
    let path_str = unsafe { CStr::from_ptr(path as *const _) }
        .to_string_lossy()
        .into_owned();
    log::debug!("IORegistryEntryFromPath(\"{}\")", path_str);

    // Ensure registry is populated (which also populates paths)
    ensure_registry();

    // Try direct lookup
    if let Some(handle) = lookup_by_path(&path_str) {
        return handle;
    }

    // Try stripping the plane prefix and matching by class name alone
    // (e.g. "IOService:/AppleACPIPlatformExpert/..." → look for the last component)
    if let Some(last) = path_str.rsplit('/').next() {
        if let Some(handle) = find_service_by_class(last) {
            return handle;
        }
    }

    0
}

/// `IORegistryEntryGetLocationInPlane(entry, plane, location_buf) → kern_return_t`
pub extern "C" fn shim_io_registry_entry_get_location(
    _entry: u32,
    _plane: *const u8,
    location: *mut u8,
) -> i32 {
    if !location.is_null() {
        let loc = b"0\0";
        unsafe {
            std::ptr::copy_nonoverlapping(loc.as_ptr(), location, loc.len());
        }
    }
    K_IO_RETURN_SUCCESS
}

// ── BSD name matching ───────────────────────────────────────────────────────

/// `IOBSDNameMatching(masterPort, options, name) → CFMutableDictionaryRef`
///
/// Creates a matching dict for a BSD device name (e.g. "disk0", "en0").
pub extern "C" fn shim_io_bsd_name_matching(
    _master_port: u32,
    _options: u32,
    name: *const u8,
) -> u64 {
    if name.is_null() {
        return 0;
    }
    let device_name = unsafe { CStr::from_ptr(name as *const _) }
        .to_string_lossy()
        .into_owned();
    log::debug!("IOBSDNameMatching(\"{}\")", device_name);

    // Create a matching dict keyed by the BSD name
    // On macOS this sets IOBSDName key in the dict — we track it like a class match
    track_matching_dict(&format!("BSD:{}", device_name))
}

/// `IOKitGetBusyState(masterPort, &busyState) → kern_return_t`
pub extern "C" fn shim_io_kit_get_busy_state(_master_port: u32, busy_state: *mut u32) -> i32 {
    if !busy_state.is_null() {
        unsafe {
            *busy_state = 0;
        } // not busy
    }
    K_IO_RETURN_SUCCESS
}

// ── Plugin interface ────────────────────────────────────────────────────────

/// `IOCreatePlugInInterfaceForService(service, pluginType, interfaceType,
///     &plugin, &score) → kern_return_t`
pub extern "C" fn shim_io_create_plugin_interface(
    _service: u32,
    _plugin_type: u64,
    _interface_type: u64,
    plugin: *mut u64,
    score: *mut i32,
) -> i32 {
    if !plugin.is_null() {
        unsafe {
            *plugin = 0;
        }
    }
    if !score.is_null() {
        unsafe {
            *score = 0;
        }
    }
    K_IO_RETURN_UNSUPPORTED
}

// ── Display info ────────────────────────────────────────────────────────────

/// `IODisplayCreateInfoDictionary(service, options) → CFDictionaryRef`
///
/// Returns a dict with display properties sourced from sysfs DRM.
pub extern "C" fn shim_io_display_create_info_dict(_service: u32, _options: u32) -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    // Try to get display info from sysfs
    #[cfg(target_os = "linux")]
    {
        // Read first DRM card's EDID or status
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.starts_with("card") || name_str == "card0" {
                    continue;
                }
                // Try to read the connector name and status
                let path = format!("/sys/class/drm/{}", name_str);
                if let Some(status) = read_sysfs(&format!("{}/status", path)) {
                    dict_set(
                        dict,
                        alloc_cfstring("DisplayConnectorStatus"),
                        alloc_cfstring(&status),
                    );
                }
                if let Some(modes) = read_sysfs(&format!("{}/modes", path)) {
                    // First line is the preferred mode
                    if let Some(mode) = modes.lines().next() {
                        dict_set(
                            dict,
                            alloc_cfstring("DisplayPreferredMode"),
                            alloc_cfstring(mode),
                        );
                    }
                }
                break;
            }
        }
    }

    // Fallback display info
    dict_set(
        dict,
        alloc_cfstring("DisplayVendorID"),
        alloc_cfnumber_i64(0),
    );
    dict_set(
        dict,
        alloc_cfstring("DisplayProductID"),
        alloc_cfnumber_i64(0),
    );
    dict_set(
        dict,
        alloc_cfstring("DisplaySerialNumber"),
        alloc_cfnumber_i64(0),
    );

    dict
}

// ── Power management — extended ─────────────────────────────────────────────

/// `IOPMAssertionCopyProperties(assertionID) → CFDictionaryRef`
pub extern "C" fn shim_iopm_assertion_copy_properties(assertion_id: u32) -> u64 {
    let guard = ACTIVE_ASSERTIONS.lock().unwrap();
    if let Some(map) = guard.as_ref() {
        if let Some((type_str, name_str)) = map.get(&assertion_id) {
            let dict = alloc_mutable_dict();
            if dict == 0 {
                return 0;
            }
            dict_set(
                dict,
                alloc_cfstring("AssertionType"),
                alloc_cfstring(type_str),
            );
            dict_set(
                dict,
                alloc_cfstring("AssertionName"),
                alloc_cfstring(name_str),
            );
            dict_set(
                dict,
                alloc_cfstring("AssertionID"),
                alloc_cfnumber_i64(assertion_id as i64),
            );
            dict_set(
                dict,
                alloc_cfstring("AssertionLevel"),
                alloc_cfnumber_i64(255),
            );
            return dict;
        }
    }
    0
}

// ── IOPSGetTimeRemainingEstimate ────────────────────────────────────────────

/// Sentinel values for IOPSGetTimeRemainingEstimate
const IOPS_TIME_REMAINING_UNKNOWN: f64 = -1.0;
const IOPS_TIME_REMAINING_UNLIMITED: f64 = -2.0;

/// `IOPSGetTimeRemainingEstimate() → CFTimeInterval`
///
/// Returns estimated battery time remaining in seconds, or:
/// -1.0 (kIOPSTimeRemainingUnknown) if on battery but unknown
/// -2.0 (kIOPSTimeRemainingUnlimited) if on AC power
pub extern "C" fn shim_iops_get_time_remaining() -> f64 {
    #[cfg(target_os = "linux")]
    {
        // Check AC status first
        if let Some(ac_online) = read_sysfs("/sys/class/power_supply/AC/online") {
            if ac_online.trim() == "1" {
                return IOPS_TIME_REMAINING_UNLIMITED;
            }
        }

        // If on battery, try to compute time remaining
        if let Some(bat_path) = first_battery_path() {
            // power_now is in microwatts, energy_now is in microwatt-hours
            if let (Some(energy_now), Some(power_now)) = (
                read_sysfs_i64(&format!("{}/energy_now", bat_path)),
                read_sysfs_i64(&format!("{}/power_now", bat_path)),
            ) {
                if power_now > 0 {
                    let hours = energy_now as f64 / power_now as f64;
                    return hours * 3600.0; // convert to seconds
                }
            }
            return IOPS_TIME_REMAINING_UNKNOWN;
        }
    }

    // No battery → unlimited (AC power)
    IOPS_TIME_REMAINING_UNLIMITED
}

// ── Extended platform expert properties ─────────────────────────────────────

/// Build properties dict for IOEthernetInterface from sysfs.
fn build_ethernet_props() -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "lo" || name_str.starts_with("veth") || name_str.starts_with("docker") {
                continue;
            }
            let path = format!("/sys/class/net/{}", name_str);
            dict_set(dict, alloc_cfstring("BSD Name"), alloc_cfstring(&name_str));
            if let Some(mac) = read_sysfs(&format!("{}/address", path)) {
                dict_set(dict, alloc_cfstring("IOMACAddress"), alloc_cfstring(&mac));
            }
            if let Some(mtu) = read_sysfs_i64(&format!("{}/mtu", path)) {
                dict_set(
                    dict,
                    alloc_cfstring("IOMaxPacketSize"),
                    alloc_cfnumber_i64(mtu),
                );
            }
            if let Some(speed) = read_sysfs_i64(&format!("{}/speed", path)) {
                dict_set(
                    dict,
                    alloc_cfstring("IOLinkSpeed"),
                    alloc_cfnumber_i64(speed * 1_000_000),
                );
            }
            if let Some(state) = read_sysfs(&format!("{}/operstate", path)) {
                dict_set(
                    dict,
                    alloc_cfstring("IOLinkStatus"),
                    alloc_cfbool(state == "up"),
                );
            }
            break; // first real NIC
        }
    }

    dict
}

/// Build properties dict for IOBlockStorageDevice from sysfs.
fn build_block_storage_props() -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/sys/class/block") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Only root devices (sda, vda, nvme0n1), not partitions
            if name_str.contains('p') && name_str.len() > 4 {
                continue;
            }
            let path = format!("/sys/class/block/{}", name_str);
            dict_set(dict, alloc_cfstring("BSD Name"), alloc_cfstring(&name_str));
            if let Some(size) = read_sysfs_i64(&format!("{}/size", path)) {
                // size is in 512-byte sectors
                dict_set(dict, alloc_cfstring("Size"), alloc_cfnumber_i64(size * 512));
            }
            if let Some(model) = read_sysfs(&format!("{}/device/model", path)) {
                dict_set(dict, alloc_cfstring("Model"), alloc_cfstring(&model));
            }
            if let Some(vendor) = read_sysfs(&format!("{}/device/vendor", path)) {
                dict_set(dict, alloc_cfstring("Vendor"), alloc_cfstring(&vendor));
            }
            if let Some(ro) = read_sysfs(&format!("{}/ro", path)) {
                dict_set(
                    dict,
                    alloc_cfstring("Writable"),
                    alloc_cfbool(ro.trim() == "0"),
                );
            }
            dict_set(dict, alloc_cfstring("Whole"), alloc_cfbool(true));
            break;
        }
    }

    dict
}

/// Build properties dict for AppleACPICPU from sysfs.
fn build_cpu_props() -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    // Number of CPU cores
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
            let count = entries
                .flatten()
                .filter(|e| {
                    let n = e.file_name();
                    let s = n.to_string_lossy();
                    s.starts_with("cpu") && s[3..].parse::<u32>().is_ok()
                })
                .count();
            dict_set(
                dict,
                alloc_cfstring("CPUCount"),
                alloc_cfnumber_i64(count as i64),
            );
        }

        // CPU frequency
        if let Some(freq) = read_sysfs_i64("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
        {
            // cpuinfo_max_freq is in kHz
            dict_set(
                dict,
                alloc_cfstring("CPUFrequency"),
                alloc_cfnumber_i64(freq * 1000),
            );
        }

        // Thermal zones
        if let Some(temp) = read_sysfs_i64("/sys/class/thermal/thermal_zone0/temp") {
            // temp is in millidegrees Celsius
            dict_set(
                dict,
                alloc_cfstring("CPUTemperature"),
                alloc_cfnumber_i64(temp),
            );
        }
    }

    // Architecture
    #[cfg(target_arch = "aarch64")]
    dict_set(
        dict,
        alloc_cfstring("CPUArchitecture"),
        alloc_cfstring("arm64"),
    );
    #[cfg(target_arch = "x86_64")]
    dict_set(
        dict,
        alloc_cfstring("CPUArchitecture"),
        alloc_cfstring("x86_64"),
    );

    dict
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_service_matching_returns_nonzero_handle() {
        let ptr = b"IOPMPowerSource\0";
        let handle = shim_io_service_matching(ptr.as_ptr());
        assert_ne!(handle, 0, "IOServiceMatching should return non-zero handle");
    }

    #[test]
    fn io_service_matching_tracks_class_name() {
        let ptr = b"IOPlatformExpertDevice\0";
        let handle = shim_io_service_matching(ptr.as_ptr());
        let class = lookup_matching_class(handle);
        assert_eq!(class.as_deref(), Some("IOPlatformExpertDevice"));
    }

    #[test]
    fn io_service_get_matching_finds_platform_expert() {
        let ptr = b"IOPlatformExpertDevice\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);
        assert_ne!(service, 0, "should find IOPlatformExpertDevice in registry");
    }

    #[test]
    fn io_service_get_matching_returns_zero_for_unknown() {
        let ptr = b"NonExistentService\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);
        assert_eq!(service, 0, "unknown class should return 0");
    }

    #[test]
    fn io_service_get_matching_services_creates_iterator() {
        let ptr = b"AppleACPIPlatformExpert\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let mut iter: u32 = 0;
        let ret = shim_io_service_get_matching_services(0, matching, &mut iter);
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
        assert_ne!(iter, 0, "iterator should be non-zero");

        // Should get at least one result
        let first = shim_io_iterator_next(iter);
        assert_ne!(first, 0, "should have at least one matching service");

        // Eventually returns 0
        let mut count = 1;
        while shim_io_iterator_next(iter) != 0 {
            count += 1;
            if count > 100 {
                break;
            }
        }
    }

    #[test]
    fn io_iterator_reset_rewinds() {
        let ptr = b"IOPlatformExpertDevice\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let mut iter: u32 = 0;
        shim_io_service_get_matching_services(0, matching, &mut iter);

        let first = shim_io_iterator_next(iter);
        // exhaust
        while shim_io_iterator_next(iter) != 0 {}

        shim_io_iterator_reset(iter);
        let first_again = shim_io_iterator_next(iter);
        assert_eq!(
            first, first_again,
            "reset should rewind to same first element"
        );
    }

    #[test]
    fn io_object_get_class_writes_name() {
        let ptr = b"IOPlatformExpertDevice\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);
        assert_ne!(service, 0);

        let mut buf = [0u8; 128];
        let ret = shim_io_object_get_class(service, buf.as_mut_ptr());
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
        let name = CStr::from_bytes_until_nul(&buf).unwrap().to_string_lossy();
        assert_eq!(name, "IOPlatformExpertDevice");
    }

    #[test]
    fn io_object_conforms_to_checks_class() {
        let ptr = b"IOPlatformExpertDevice\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);
        assert_ne!(service, 0);

        assert_eq!(
            shim_io_object_conforms_to(service, c"IOPlatformExpertDevice".as_ptr().cast()),
            1
        );
        assert_eq!(
            shim_io_object_conforms_to(service, c"IOHIDDevice".as_ptr().cast()),
            0
        );
    }

    #[test]
    fn io_registry_entry_get_name_writes_class() {
        let ptr = b"AppleACPIPlatformExpert\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);

        let mut buf = [0u8; 128];
        let ret = shim_io_registry_entry_get_name(service, buf.as_mut_ptr());
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
    }

    #[test]
    fn io_registry_entry_get_path_writes_path() {
        let ptr = b"IOPlatformExpertDevice\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);

        let mut buf = [0u8; 512];
        let ret = shim_io_registry_entry_get_path(service, std::ptr::null(), buf.as_mut_ptr());
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
        let path = CStr::from_bytes_until_nul(&buf).unwrap().to_string_lossy();
        assert!(
            path.starts_with("IOService:/"),
            "path should start with IOService:/"
        );
    }

    #[test]
    fn io_registry_entry_create_cf_property_serial_number() {
        let ptr = b"IOPlatformExpertDevice\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);
        assert_ne!(service, 0);

        let key = alloc_cfstring("IOPlatformSerialNumber");
        let val = shim_io_registry_entry_create_cf_property(service, key, 0, 0);
        assert_ne!(val, 0, "serial number property should be non-NULL");
    }

    #[test]
    fn io_registry_entry_create_cf_properties_returns_dict() {
        let ptr = b"IOPlatformExpertDevice\0";
        let matching = shim_io_service_matching(ptr.as_ptr());
        let service = shim_io_service_get_matching_service(0, matching);
        assert_ne!(service, 0);

        let mut props: u64 = 0;
        let ret = shim_io_registry_entry_create_cf_properties(service, &mut props, 0, 0);
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
        assert_ne!(props, 0, "properties dict should be non-NULL");
    }

    #[test]
    fn io_registry_entry_create_cf_properties_not_found() {
        let mut props: u64 = 0;
        let ret = shim_io_registry_entry_create_cf_properties(0xDEAD, &mut props, 0, 0);
        assert_eq!(ret, K_IO_RETURN_NOT_FOUND);
    }

    #[test]
    fn io_registry_get_root_entry_nonzero() {
        let root = shim_io_registry_get_root_entry(0);
        assert_ne!(root, 0, "root entry should be non-zero");
    }

    #[test]
    fn io_master_port_returns_success() {
        let mut port: u32 = 0xFF;
        let ret = shim_io_master_port(0, &mut port);
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
        assert_eq!(port, 0);
    }

    #[test]
    fn iopm_assertion_create_unique_ids() {
        let type_str = b"NoIdleSleepAssertion\0";
        let name_str = b"test\0";
        let mut id1: u32 = 0;
        let mut id2: u32 = 0;
        assert_eq!(
            shim_iopm_assertion_create(
                type_str.as_ptr() as u64,
                255,
                name_str.as_ptr() as u64,
                &mut id1
            ),
            K_IO_RETURN_SUCCESS
        );
        assert_eq!(
            shim_iopm_assertion_create(
                type_str.as_ptr() as u64,
                255,
                name_str.as_ptr() as u64,
                &mut id2
            ),
            K_IO_RETURN_SUCCESS
        );
        assert_ne!(id1, id2, "assertion IDs should be unique");
    }

    #[test]
    fn iopm_assertion_release_removes() {
        let type_str = b"Test\0";
        let name_str = b"test\0";
        let mut id: u32 = 0;
        shim_iopm_assertion_create(
            type_str.as_ptr() as u64,
            255,
            name_str.as_ptr() as u64,
            &mut id,
        );

        let ret = shim_iopm_assertion_release(id);
        assert_eq!(ret, K_IO_RETURN_SUCCESS);

        // Verify it was removed
        let guard = ACTIVE_ASSERTIONS.lock().unwrap();
        if let Some(map) = guard.as_ref() {
            assert!(!map.contains_key(&id), "assertion should be removed");
        }
    }

    #[test]
    fn io_return_unsupported_value() {
        assert_eq!(K_IO_RETURN_UNSUPPORTED, 0xe00002c7u32 as i32);
    }

    #[test]
    fn io_return_not_found_value() {
        assert_eq!(K_IO_RETURN_NOT_FOUND, 0xe00002beu32 as i32);
    }

    #[test]
    fn alloc_cfstring_roundtrip() {
        let s = "Hello, IOKit!";
        let ptr = alloc_cfstring(s);
        assert_ne!(ptr, 0);

        // Verify we can read the string back via extract_key_string
        let result = extract_key_string(ptr);
        assert_eq!(result.as_deref(), Some(s));
    }

    #[test]
    fn io_notification_port_create_nonzero() {
        let port = shim_io_notification_port_create(0);
        assert_ne!(port, 0);
    }

    #[test]
    fn iops_copy_power_sources_info_nonzero() {
        let info = shim_iops_copy_power_sources_info();
        assert_ne!(info, 0);
    }

    #[test]
    fn iops_get_providing_type_returns_string() {
        let s = shim_iops_get_providing_type(0);
        assert_ne!(s, 0, "should return a power source type string");
    }

    #[test]
    fn data_symbols_have_stable_addresses() {
        let port_ptr = &K_IO_MASTER_PORT_DEFAULT as *const u32;
        assert_eq!(unsafe { *port_ptr }, 0);

        // Assertion type constants should be non-null NUL-terminated strings
        let no_idle = IOPM_NO_IDLE_SLEEP;
        assert!(no_idle.len() > 1);
        assert_eq!(*no_idle.last().unwrap(), 0);
    }

    #[test]
    fn io_service_open_returns_unique_handles() {
        let mut connect1: u32 = 0;
        let mut connect2: u32 = 0;
        shim_io_service_open(1, 0, 0, &mut connect1);
        shim_io_service_open(1, 0, 0, &mut connect2);
        assert_ne!(connect1, 0);
        assert_ne!(connect2, 0);
        assert_ne!(
            connect1, connect2,
            "each open should return a unique handle"
        );
    }

    #[test]
    fn io_registry_entry_search_property_walks_registry() {
        // IOPlatformSerialNumber should be found even when searching from root
        let root = shim_io_registry_get_root_entry(0);
        let key = alloc_cfstring("IOPlatformSerialNumber");
        let val = shim_io_registry_entry_search_cf_property(root, std::ptr::null(), key, 0, 0);
        assert_ne!(val, 0, "search should find serial number from any entry");
    }

    #[test]
    fn build_power_source_dict_returns_nonempty() {
        let dict = build_power_source_dict();
        assert_ne!(dict, 0, "power source dict should be non-NULL");
        // Should have at least the "Is Present" and "Power Source State" keys
        let count = unsafe { *((dict as usize + 8) as *const u64) };
        assert!(
            count >= 2,
            "dict should have at least 2 keys, got {}",
            count
        );
    }

    // ── New tests for extended IOKit functions ──────────────────────────────

    #[test]
    fn io_registry_entry_get_id_returns_unique() {
        let h1 = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let h2 = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut id1: u64 = 0;
        let mut id2: u64 = 0;
        let ret1 = shim_io_registry_entry_get_id(h1, &mut id1);
        let ret2 = shim_io_registry_entry_get_id(h2, &mut id2);
        assert_eq!(ret1, K_IO_RETURN_SUCCESS);
        assert_eq!(ret2, K_IO_RETURN_SUCCESS);
        assert_ne!(id1, 0);
        assert_ne!(id2, 0);
        assert_ne!(id1, id2, "different handles should get different IDs");
    }

    #[test]
    fn io_registry_entry_get_id_stable() {
        let h = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut id1: u64 = 0;
        let mut id2: u64 = 0;
        shim_io_registry_entry_get_id(h, &mut id1);
        shim_io_registry_entry_get_id(h, &mut id2);
        assert_eq!(id1, id2, "same handle should always return the same ID");
    }

    #[test]
    fn io_registry_entry_get_id_null_returns_error() {
        let ret = shim_io_registry_entry_get_id(1, std::ptr::null_mut());
        assert_eq!(ret, K_IO_RETURN_UNSUPPORTED);
    }

    #[test]
    fn io_registry_entry_from_path_null_returns_zero() {
        let h = shim_io_registry_entry_from_path(0, std::ptr::null());
        assert_eq!(h, 0);
    }

    #[test]
    fn io_registry_entry_get_location_writes_nul_terminated() {
        let mut buf = [0xFFu8; 16];
        let ret = shim_io_registry_entry_get_location(1, std::ptr::null(), buf.as_mut_ptr());
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
        assert_eq!(buf[0], b'0');
        assert_eq!(buf[1], 0);
    }

    #[test]
    fn io_bsd_name_matching_returns_nonzero() {
        let name = b"disk0\0";
        let dict = shim_io_bsd_name_matching(0, 0, name.as_ptr());
        assert_ne!(dict, 0, "BSD name matching should return a valid dict");
    }

    #[test]
    fn io_bsd_name_matching_null_returns_zero() {
        let dict = shim_io_bsd_name_matching(0, 0, std::ptr::null());
        assert_eq!(dict, 0);
    }

    #[test]
    fn io_kit_get_busy_state_zero() {
        let mut busy: u32 = 99;
        let ret = shim_io_kit_get_busy_state(0, &mut busy);
        assert_eq!(ret, K_IO_RETURN_SUCCESS);
        assert_eq!(busy, 0, "system should report not busy");
    }

    #[test]
    fn io_create_plugin_interface_unsupported() {
        let mut plugin: u64 = 0xDEAD;
        let mut score: i32 = 99;
        let ret = shim_io_create_plugin_interface(1, 0, 0, &mut plugin, &mut score);
        assert_eq!(ret, K_IO_RETURN_UNSUPPORTED);
        assert_eq!(plugin, 0, "plugin should be cleared to NULL");
        assert_eq!(score, 0);
    }

    #[test]
    fn io_display_create_info_dict_nonzero() {
        let dict = shim_io_display_create_info_dict(1, 0);
        assert_ne!(dict, 0, "display info dict should be non-NULL");
    }

    #[test]
    fn iopm_assertion_copy_properties_returns_dict() {
        let type_str = b"PreventUserIdleSystemSleep\0";
        let name_str = b"TestCopy\0";
        let mut id: u32 = 0;
        shim_iopm_assertion_create(
            type_str.as_ptr() as u64,
            255,
            name_str.as_ptr() as u64,
            &mut id,
        );
        assert_ne!(id, 0);

        let dict = shim_iopm_assertion_copy_properties(id);
        assert_ne!(
            dict, 0,
            "copy properties should return non-NULL for active assertion"
        );

        // clean up
        shim_iopm_assertion_release(id);
    }

    #[test]
    fn iopm_assertion_copy_properties_unknown_returns_null() {
        let dict = shim_iopm_assertion_copy_properties(0xDEADBEEF);
        assert_eq!(dict, 0, "should return NULL for unknown assertion ID");
    }

    #[test]
    fn iops_get_time_remaining_returns_valid() {
        let result = shim_iops_get_time_remaining();
        // Must be either a positive time, -1 (unknown), or -2 (unlimited/AC)
        assert!(
            result > 0.0
                || result == IOPS_TIME_REMAINING_UNKNOWN
                || result == IOPS_TIME_REMAINING_UNLIMITED,
            "unexpected time remaining value: {}",
            result,
        );
    }

    #[test]
    fn io_return_unsupported_stub_returns_error() {
        let ret = shim_io_return_unsupported_stub(0, 0, 0, 0);
        assert_eq!(ret, K_IO_RETURN_UNSUPPORTED);
    }

    #[test]
    fn build_ethernet_props_nonzero() {
        let dict = build_ethernet_props();
        assert_ne!(dict, 0, "ethernet props dict should be non-NULL");
    }

    #[test]
    fn build_cpu_props_nonzero() {
        let dict = build_cpu_props();
        assert_ne!(dict, 0, "CPU props dict should be non-NULL");
    }

    #[test]
    fn build_block_storage_props_nonzero() {
        let dict = build_block_storage_props();
        assert_ne!(dict, 0, "block storage props dict should be non-NULL");
    }
}
