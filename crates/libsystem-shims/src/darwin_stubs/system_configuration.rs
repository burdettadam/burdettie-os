//! Apple SystemConfiguration.framework — real network state on Linux.
//!
//! SCNetworkReachability does a real connectivity check via getaddrinfo.
//! SCDynamicStore returns real network state from Linux sysfs/procfs.
//! SCNetworkInterfaceCopyAll returns real interfaces from /sys/class/net/.
//! SCPreferences are in-memory key-value stubs.

use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::table::ShimTable;

/// Monotonic handle generator — every Create call returns a unique non-zero value.
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn next_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// ── SCNetworkReachabilityFlags constants ────────────────────────
/// Apple SCNetworkReachabilityFlags values.
const K_SC_FLAG_REACHABLE: u32 = 1 << 1; // 2
#[allow(dead_code)]
const K_SC_FLAG_CONNECTION_REQUIRED: u32 = 1 << 2; // 4
const K_SC_FLAG_IS_DIRECT: u32 = 1 << 17; // 131072
const K_SC_FLAG_IS_LOCAL_ADDRESS: u32 = 1 << 16; // 65536

/// Data symbol for kSCNetworkReachabilityFlagsReachable.
static K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE: u32 = 2;

// ── SC key string constants (NUL-terminated) ────────────────────

/// kSCDynamicStoreDomainState = "State"
static KSCDYNAMIC_STORE_DOMAIN_STATE: &[u8] = b"State\0";
/// kSCDynamicStoreDomainSetup = "Setup"
static KSCDYNAMIC_STORE_DOMAIN_SETUP: &[u8] = b"Setup\0";
/// kSCCompNetwork = "Network"
static KSCCOMP_NETWORK: &[u8] = b"Network\0";
/// kSCCompInterface = "Interface"
static KSCCOMP_INTERFACE: &[u8] = b"Interface\0";
/// kSCCompService = "Service"
static KSCCOMP_SERVICE: &[u8] = b"Service\0";
/// kSCCompGlobal = "Global"
static KSCCOMP_GLOBAL: &[u8] = b"Global\0";
/// kSCEntNetIPv4 = "IPv4"
static KSCENT_NET_IPV4: &[u8] = b"IPv4\0";
/// kSCEntNetIPv6 = "IPv6"
static KSCENT_NET_IPV6: &[u8] = b"IPv6\0";
/// kSCEntNetDNS = "DNS"
static KSCENT_NET_DNS: &[u8] = b"DNS\0";
/// kSCEntNetProxies = "Proxies"
static KSCENT_NET_PROXIES: &[u8] = b"Proxies\0";
/// kSCEntNetLink = "Link"
static KSCENT_NET_LINK: &[u8] = b"Link\0";
/// kSCEntNetAirPort = "AirPort"
static KSCENT_NET_AIRPORT: &[u8] = b"AirPort\0";
/// kSCEntNetEthernet = "Ethernet"
static KSCENT_NET_ETHERNET: &[u8] = b"Ethernet\0";
/// kSCPropNetIPv4Addresses = "Addresses"
static KSCPROP_NET_IPV4_ADDRESSES: &[u8] = b"Addresses\0";
/// kSCPropNetIPv4Router = "Router"
static KSCPROP_NET_IPV4_ROUTER: &[u8] = b"Router\0";
/// kSCPropNetIPv4SubnetMasks = "SubnetMasks"
static KSCPROP_NET_IPV4_SUBNET_MASKS: &[u8] = b"SubnetMasks\0";
/// kSCPropNetIPv6Addresses = "Addresses"
static KSCPROP_NET_IPV6_ADDRESSES: &[u8] = b"Addresses\0";
/// kSCPropNetDNSServerAddresses = "ServerAddresses"
static KSCPROP_NET_DNS_SERVER_ADDRESSES: &[u8] = b"ServerAddresses\0";
/// kSCPropNetDNSSearchDomains = "SearchDomains"
static KSCPROP_NET_DNS_SEARCH_DOMAINS: &[u8] = b"SearchDomains\0";
/// kSCPropInterfaceName = "InterfaceName"
static KSCPROP_INTERFACE_NAME: &[u8] = b"InterfaceName\0";
/// kSCPropNetInterfaceType = "Type"
static KSCPROP_NET_INTERFACE_TYPE: &[u8] = b"Type\0";
/// kSCPropNetInterfaceHardware = "Hardware"
static KSCPROP_NET_INTERFACE_HARDWARE: &[u8] = b"Hardware\0";
/// kSCValNetInterfaceTypeEthernet = "Ethernet"
static KSCVAL_NET_INTERFACE_TYPE_ETHERNET: &[u8] = b"Ethernet\0";
/// kSCPropNetLinkActive = "Active"
static KSCPROP_NET_LINK_ACTIVE: &[u8] = b"Active\0";
/// kSCDynamicStorePropNetPrimaryInterface = "PrimaryInterface"
static KSCPROP_NET_PRIMARY_INTERFACE: &[u8] = b"PrimaryInterface\0";
/// kSCDynamicStorePropNetPrimaryService = "PrimaryService"
static KSCPROP_NET_PRIMARY_SERVICE: &[u8] = b"PrimaryService\0";

// ── Reachability store ──────────────────────────────────────────

/// Stored reachability target info.
struct ReachabilityTarget {
    hostname: Option<String>,
    /// Cached flags from last check
    #[allow(dead_code)]
    flags: u32,
}

static REACHABILITY_STORE: Mutex<Option<HashMap<u64, ReachabilityTarget>>> = Mutex::new(None);

fn reachability_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, ReachabilityTarget>>>
{
    REACHABILITY_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

// ── SCDynamicStore backing store ────────────────────────────────

/// Per-store state for SCDynamicStore instances.
struct DynStoreState {
    /// Notification keys (patterns) registered via SetNotificationKeys.
    #[allow(dead_code)]
    notification_keys: Vec<String>,
    /// Callback function pointer (never called, but tracked).
    #[allow(dead_code)]
    callback: u64,
}

static DYNSTORE_STATE: Mutex<Option<HashMap<u64, DynStoreState>>> = Mutex::new(None);

fn dynstore_state() -> std::sync::MutexGuard<'static, Option<HashMap<u64, DynStoreState>>> {
    DYNSTORE_STATE.lock().unwrap_or_else(|e| e.into_inner())
}

// ── SCPreferences backing store ─────────────────────────────────

static PREFS_STORE: Mutex<Option<HashMap<u64, HashMap<String, u64>>>> = Mutex::new(None);

fn prefs_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, HashMap<String, u64>>>> {
    PREFS_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct SystemConfigurationStubs;

impl SystemConfigurationStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── SCNetworkReachability ───────────────────────────────
        register_shims!(
            table,
            (
                "_SCNetworkReachabilityCreateWithName",
                shim_sc_reachability_create_with_name as *const () as u64,
                "SCNetworkReachabilityCreateWithName → handle"
            ),
            (
                "_SCNetworkReachabilityCreateWithAddress",
                shim_sc_reachability_create_with_address as *const () as u64,
                "SCNetworkReachabilityCreateWithAddress → handle"
            ),
            (
                "_SCNetworkReachabilityCreateWithAddressPair",
                shim_sc_reachability_create_with_address_pair as *const () as u64,
                "SCNetworkReachabilityCreateWithAddressPair → handle"
            ),
            (
                "_SCNetworkReachabilityGetFlags",
                shim_sc_reachability_get_flags as *const () as u64,
                "SCNetworkReachabilityGetFlags → real check"
            ),
            (
                "_SCNetworkReachabilitySetCallback",
                shim_sc_reachability_set_callback as *const () as u64,
                "SCNetworkReachabilitySetCallback → TRUE"
            ),
            (
                "_SCNetworkReachabilityScheduleWithRunLoop",
                shim_sc_reachability_schedule_with_run_loop as *const () as u64,
                "SCNetworkReachabilityScheduleWithRunLoop → TRUE"
            ),
            (
                "_SCNetworkReachabilityUnscheduleFromRunLoop",
                shim_sc_reachability_unschedule_from_run_loop as *const () as u64,
                "SCNetworkReachabilityUnscheduleFromRunLoop → TRUE"
            ),
            (
                "_SCNetworkReachabilitySetDispatchQueue",
                shim_sc_reachability_set_dispatch_queue as *const () as u64,
                "SCNetworkReachabilitySetDispatchQueue → TRUE"
            )
        );

        // ── SCDynamicStore ──────────────────────────────────────
        register_shims!(
            table,
            (
                "_SCDynamicStoreCreate",
                shim_sc_dynamic_store_create as *const () as u64,
                "SCDynamicStoreCreate → handle"
            ),
            (
                "_SCDynamicStoreCopyValue",
                shim_sc_dynamic_store_copy_value as *const () as u64,
                "SCDynamicStoreCopyValue → real network data"
            ),
            (
                "_SCDynamicStoreCopyMultiple",
                shim_sc_dynamic_store_copy_multiple as *const () as u64,
                "SCDynamicStoreCopyMultiple → dict"
            ),
            (
                "_SCDynamicStoreSetValue",
                shim_sc_dynamic_store_set_value as *const () as u64,
                "SCDynamicStoreSetValue → TRUE"
            ),
            (
                "_SCDynamicStoreSetNotificationKeys",
                shim_sc_dynamic_store_set_notification_keys as *const () as u64,
                "SCDynamicStoreSetNotificationKeys → TRUE"
            ),
            (
                "_SCDynamicStoreSetDispatchQueue",
                shim_sc_dynamic_store_set_dispatch_queue as *const () as u64,
                "SCDynamicStoreSetDispatchQueue → TRUE"
            ),
            (
                "_SCDynamicStoreCopyKeyList",
                shim_sc_dynamic_store_copy_key_list as *const () as u64,
                "SCDynamicStoreCopyKeyList → array"
            ),
            (
                "_SCDynamicStoreCreateRunLoopSource",
                shim_sc_dynamic_store_create_run_loop_source as *const () as u64,
                "SCDynamicStoreCreateRunLoopSource → handle"
            ),
            (
                "_SCDynamicStoreNotifyValue",
                shim_sc_dynamic_store_notify_value as *const () as u64,
                "SCDynamicStoreNotifyValue → TRUE"
            )
        );

        // ── SCPreferences ───────────────────────────────────────
        register_shims!(
            table,
            (
                "_SCPreferencesCreate",
                shim_sc_preferences_create as *const () as u64,
                "SCPreferencesCreate → handle"
            ),
            (
                "_SCPreferencesGetValue",
                shim_sc_preferences_get_value as *const () as u64,
                "SCPreferencesGetValue → value"
            ),
            (
                "_SCPreferencesSetValue",
                shim_sc_preferences_set_value as *const () as u64,
                "SCPreferencesSetValue → TRUE"
            ),
            (
                "_SCPreferencesCommitChanges",
                shim_sc_preferences_commit_changes as *const () as u64,
                "SCPreferencesCommitChanges → TRUE"
            ),
            (
                "_SCPreferencesApplyChanges",
                shim_sc_preferences_apply_changes as *const () as u64,
                "SCPreferencesApplyChanges → TRUE"
            ),
            (
                "_SCPreferencesPathGetValue",
                shim_sc_preferences_path_get_value as *const () as u64,
                "SCPreferencesPathGetValue → NULL"
            ),
            (
                "_SCPreferencesLock",
                shim_sc_preferences_lock as *const () as u64,
                "SCPreferencesLock → TRUE"
            ),
            (
                "_SCPreferencesUnlock",
                shim_sc_preferences_unlock as *const () as u64,
                "SCPreferencesUnlock → TRUE"
            )
        );

        // ── SCNetworkConfiguration ──────────────────────────────
        register_shims!(
            table,
            (
                "_SCNetworkInterfaceCopyAll",
                shim_sc_network_interface_copy_all as *const () as u64,
                "SCNetworkInterfaceCopyAll → real interfaces"
            ),
            (
                "_SCNetworkServiceCopyAll",
                shim_sc_network_service_copy_all as *const () as u64,
                "SCNetworkServiceCopyAll → NULL"
            ),
            (
                "_SCNetworkInterfaceGetBSDName",
                shim_sc_network_interface_get_bsd_name as *const () as u64,
                "SCNetworkInterfaceGetBSDName → name"
            ),
            (
                "_SCNetworkInterfaceGetInterfaceType",
                shim_sc_network_interface_get_interface_type as *const () as u64,
                "SCNetworkInterfaceGetInterfaceType → type"
            ),
            (
                "_SCNetworkInterfaceGetHardwareAddressString",
                shim_sc_network_interface_get_hw_addr as *const () as u64,
                "SCNetworkInterfaceGetHardwareAddressString → MAC"
            ),
            (
                "_SCNetworkInterfaceGetLocalizedDisplayName",
                shim_sc_network_interface_get_display_name as *const () as u64,
                "SCNetworkInterfaceGetLocalizedDisplayName → name"
            ),
            (
                "_SCNetworkInterfaceGetConfiguration",
                shim_sc_network_interface_get_configuration as *const () as u64,
                "SCNetworkInterfaceGetConfiguration → NULL"
            )
        );

        // ── Error handling ──────────────────────────────────────
        register_shims!(
            table,
            ("_SCError", shim_sc_error as *const () as u64, "SCError → 0"),
            (
                "_SCErrorString",
                shim_sc_error_string as *const () as u64,
                "SCErrorString → string"
            ),
            (
                "_SCCopyLastError",
                shim_sc_copy_last_error as *const () as u64,
                "SCCopyLastError → NULL"
            )
        );

        // ── Data symbols: key string constants ──────────────────
        register_shims!(
            table,
            (
                "_kSCNetworkReachabilityFlagsReachable",
                &K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE as *const u32 as u64,
                "kSCNetworkReachabilityFlagsReachable"
            ),
            (
                "_kSCDynamicStoreDomainState",
                KSCDYNAMIC_STORE_DOMAIN_STATE.as_ptr() as u64,
                "kSCDynamicStoreDomainState"
            ),
            (
                "_kSCDynamicStoreDomainSetup",
                KSCDYNAMIC_STORE_DOMAIN_SETUP.as_ptr() as u64,
                "kSCDynamicStoreDomainSetup"
            ),
            (
                "_kSCCompNetwork",
                KSCCOMP_NETWORK.as_ptr() as u64,
                "kSCCompNetwork"
            ),
            (
                "_kSCCompInterface",
                KSCCOMP_INTERFACE.as_ptr() as u64,
                "kSCCompInterface"
            ),
            (
                "_kSCCompService",
                KSCCOMP_SERVICE.as_ptr() as u64,
                "kSCCompService"
            ),
            (
                "_kSCCompGlobal",
                KSCCOMP_GLOBAL.as_ptr() as u64,
                "kSCCompGlobal"
            ),
            (
                "_kSCEntNetIPv4",
                KSCENT_NET_IPV4.as_ptr() as u64,
                "kSCEntNetIPv4"
            ),
            (
                "_kSCEntNetIPv6",
                KSCENT_NET_IPV6.as_ptr() as u64,
                "kSCEntNetIPv6"
            ),
            (
                "_kSCEntNetDNS",
                KSCENT_NET_DNS.as_ptr() as u64,
                "kSCEntNetDNS"
            ),
            (
                "_kSCEntNetProxies",
                KSCENT_NET_PROXIES.as_ptr() as u64,
                "kSCEntNetProxies"
            ),
            (
                "_kSCEntNetLink",
                KSCENT_NET_LINK.as_ptr() as u64,
                "kSCEntNetLink"
            ),
            (
                "_kSCEntNetAirPort",
                KSCENT_NET_AIRPORT.as_ptr() as u64,
                "kSCEntNetAirPort"
            ),
            (
                "_kSCEntNetEthernet",
                KSCENT_NET_ETHERNET.as_ptr() as u64,
                "kSCEntNetEthernet"
            ),
            (
                "_kSCPropNetIPv4Addresses",
                KSCPROP_NET_IPV4_ADDRESSES.as_ptr() as u64,
                "kSCPropNetIPv4Addresses"
            ),
            (
                "_kSCPropNetIPv4Router",
                KSCPROP_NET_IPV4_ROUTER.as_ptr() as u64,
                "kSCPropNetIPv4Router"
            ),
            (
                "_kSCPropNetIPv4SubnetMasks",
                KSCPROP_NET_IPV4_SUBNET_MASKS.as_ptr() as u64,
                "kSCPropNetIPv4SubnetMasks"
            ),
            (
                "_kSCPropNetIPv6Addresses",
                KSCPROP_NET_IPV6_ADDRESSES.as_ptr() as u64,
                "kSCPropNetIPv6Addresses"
            ),
            (
                "_kSCPropNetDNSServerAddresses",
                KSCPROP_NET_DNS_SERVER_ADDRESSES.as_ptr() as u64,
                "kSCPropNetDNSServerAddresses"
            ),
            (
                "_kSCPropNetDNSSearchDomains",
                KSCPROP_NET_DNS_SEARCH_DOMAINS.as_ptr() as u64,
                "kSCPropNetDNSSearchDomains"
            ),
            (
                "_kSCPropInterfaceName",
                KSCPROP_INTERFACE_NAME.as_ptr() as u64,
                "kSCPropInterfaceName"
            ),
            (
                "_kSCPropNetInterfaceType",
                KSCPROP_NET_INTERFACE_TYPE.as_ptr() as u64,
                "kSCPropNetInterfaceType"
            ),
            (
                "_kSCPropNetInterfaceHardware",
                KSCPROP_NET_INTERFACE_HARDWARE.as_ptr() as u64,
                "kSCPropNetInterfaceHardware"
            ),
            (
                "_kSCValNetInterfaceTypeEthernet",
                KSCVAL_NET_INTERFACE_TYPE_ETHERNET.as_ptr() as u64,
                "kSCValNetInterfaceTypeEthernet"
            ),
            (
                "_kSCPropNetLinkActive",
                KSCPROP_NET_LINK_ACTIVE.as_ptr() as u64,
                "kSCPropNetLinkActive"
            ),
            (
                "_kSCDynamicStorePropNetPrimaryInterface",
                KSCPROP_NET_PRIMARY_INTERFACE.as_ptr() as u64,
                "kSCDynamicStorePropNetPrimaryInterface"
            ),
            (
                "_kSCDynamicStorePropNetPrimaryService",
                KSCPROP_NET_PRIMARY_SERVICE.as_ptr() as u64,
                "kSCDynamicStorePropNetPrimaryService"
            )
        );

        log::debug!("SystemConfigurationStubs: registered 63 shims + 27 data symbols");
    }
}

// ── CF object helpers (same layout as IOKit) ────────────────────

/// Allocate a CFString-compatible object (ISA=0x1, layout: [isa:8][len:8][ptr:8][pad:8][chars...]).
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

/// Allocate a CFNumber-compatible object (ISA=0x2, layout: [isa:8][value:8]).
fn alloc_cfnumber_i64(val: i64) -> u64 {
    let ptr = unsafe { libc::calloc(1, 16) } as u64;
    if ptr == 0 {
        return 0;
    }
    unsafe {
        let isa = 0x2u64;
        std::ptr::copy_nonoverlapping(&isa as *const u64 as *const u8, ptr as *mut u8, 8);
        std::ptr::copy_nonoverlapping(&val as *const i64 as *const u8, (ptr as *mut u8).add(8), 8);
    }
    ptr
}

/// Allocate a CFDictionary-compatible mutable object.
/// Layout: [isa:8][count:8][keys_ptr:8][vals_ptr:8][capacity:8] = 40 bytes.
fn alloc_mutable_dict() -> u64 {
    let ptr = unsafe { libc::calloc(1, 40) } as u64;
    if ptr != 0 {
        unsafe {
            let isa = 0x3u64;
            std::ptr::copy_nonoverlapping(&isa as *const u64 as *const u8, ptr as *mut u8, 8);
        }
    }
    ptr
}

/// Add a key-value pair to a mutable dict.
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

/// Allocate a CFArray-compatible object.
/// Layout: [isa:8][count:8][values_ptr:8] = 24 bytes.
fn alloc_cfarray(items: &[u64]) -> u64 {
    let ptr = unsafe { libc::calloc(1, 24) } as u64;
    if ptr == 0 {
        return 0;
    }
    let count = items.len();
    let vals = unsafe { libc::calloc(count, 8) } as *mut u64;
    if vals.is_null() {
        return ptr; // empty array
    }
    unsafe {
        let isa = 0x4u64;
        std::ptr::copy_nonoverlapping(&isa as *const u64 as *const u8, ptr as *mut u8, 8);
        for (i, item) in items.iter().enumerate() {
            *vals.add(i) = *item;
        }
        let base = ptr as *mut u8;
        let count_u64 = count as u64;
        std::ptr::copy_nonoverlapping(&count_u64 as *const u64 as *const u8, base.add(8), 8);
        let vals_u64 = vals as u64;
        std::ptr::copy_nonoverlapping(&vals_u64 as *const u64 as *const u8, base.add(16), 8);
    }
    ptr
}

/// Read a sysfs/procfs text file, trimming whitespace.
fn read_sysfs(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Extract a CFString key from a pointer (reads [isa][len][ptr] layout).
fn extract_key_string(key_ptr: u64) -> Option<String> {
    if key_ptr == 0 {
        return None;
    }
    unsafe {
        let buf = key_ptr as *const u8;
        let len = std::ptr::read_unaligned(buf.add(8) as *const u64) as usize;
        let chars_ptr = std::ptr::read_unaligned(buf.add(16) as *const u64) as *const u8;
        if chars_ptr.is_null() || len == 0 || len > 4096 {
            // Might be a C string pointer — try reading directly
            let cstr = CStr::from_ptr(key_ptr as *const _);
            return cstr.to_str().ok().map(|s| s.to_string());
        }
        let bytes = std::slice::from_raw_parts(chars_ptr, len);
        String::from_utf8(bytes.to_vec()).ok()
    }
}

// ── Linux network state readers ─────────────────────────────────

/// Get the primary (default route) interface name from /proc/net/route.
fn get_primary_interface() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let contents = std::fs::read_to_string("/proc/net/route").ok()?;
        for line in contents.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 2 && fields[1] == "00000000" {
                // Destination 0.0.0.0 = default route
                return Some(fields[0].to_string());
            }
        }
    }
    None
}

/// Get IPv4 addresses for an interface from /proc/net/fib_trie or ip command.
fn get_ipv4_addresses(iface: &str) -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        let mut addrs = Vec::new();
        // Read from /sys/class/net/{iface}/.. is not available for IPs,
        // fall back to parsing /proc/net/fib_trie
        if let Ok(output) = std::process::Command::new("ip")
            .args(["-4", "addr", "show", iface])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("inet ") {
                    // "inet 192.168.1.100/24 ..."
                    if let Some(addr_cidr) = trimmed.split_whitespace().nth(1) {
                        if let Some(addr) = addr_cidr.split('/').next() {
                            addrs.push(addr.to_string());
                        }
                    }
                }
            }
        }
        addrs
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = iface;
        Vec::new()
    }
}

/// Get IPv6 addresses for an interface.
fn get_ipv6_addresses(iface: &str) -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        let mut addrs = Vec::new();
        if let Ok(output) = std::process::Command::new("ip")
            .args(["-6", "addr", "show", iface])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("inet6 ") {
                    if let Some(addr_cidr) = trimmed.split_whitespace().nth(1) {
                        if let Some(addr) = addr_cidr.split('/').next() {
                            addrs.push(addr.to_string());
                        }
                    }
                }
            }
        }
        addrs
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = iface;
        Vec::new()
    }
}

/// Get the default gateway IP.
fn get_default_gateway() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let contents = std::fs::read_to_string("/proc/net/route").ok()?;
        for line in contents.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 3 && fields[1] == "00000000" {
                // Gateway is hex-encoded in little-endian
                if let Ok(gw) = u32::from_str_radix(fields[2], 16) {
                    let bytes = gw.to_le_bytes();
                    return Some(format!(
                        "{}.{}.{}.{}",
                        bytes[0], bytes[1], bytes[2], bytes[3]
                    ));
                }
            }
        }
    }
    None
}

/// Get subnet mask for the primary interface.
fn get_subnet_mask(iface: &str) -> Option<String> {
    #[cfg(target_os = "linux")]
    if let Ok(output) = std::process::Command::new("ip")
        .args(["-4", "addr", "show", iface])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("inet ") {
                if let Some(addr_cidr) = trimmed.split_whitespace().nth(1) {
                    if let Some(prefix_str) = addr_cidr.split('/').nth(1) {
                        if let Ok(prefix) = prefix_str.parse::<u32>() {
                            let mask = if prefix == 0 {
                                0u32
                            } else {
                                !0u32 << (32 - prefix)
                            };
                            let bytes = mask.to_be_bytes();
                            return Some(format!(
                                "{}.{}.{}.{}",
                                bytes[0], bytes[1], bytes[2], bytes[3]
                            ));
                        }
                    }
                }
            }
        }
    }

    let _ = iface;
    None
}

/// Get DNS server addresses from /etc/resolv.conf.
fn get_dns_servers() -> Vec<String> {
    let mut servers = Vec::new();
    if let Ok(contents) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("nameserver ") {
                if let Some(addr) = trimmed.split_whitespace().nth(1) {
                    servers.push(addr.to_string());
                }
            }
        }
    }
    servers
}

/// Get DNS search domains from /etc/resolv.conf.
fn get_dns_search_domains() -> Vec<String> {
    let mut domains = Vec::new();
    if let Ok(contents) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("search ") || trimmed.starts_with("domain ") {
                for domain in trimmed.split_whitespace().skip(1) {
                    domains.push(domain.to_string());
                }
            }
        }
    }
    domains
}

/// Check if an interface link is active.
fn is_link_active(iface: &str) -> bool {
    read_sysfs(&format!("/sys/class/net/{}/operstate", iface))
        .map(|s| s == "up")
        .unwrap_or(false)
}

/// Get MAC address for an interface.
fn get_mac_address(iface: &str) -> Option<String> {
    read_sysfs(&format!("/sys/class/net/{}/address", iface))
}

/// Get MTU for an interface.
fn get_mtu(iface: &str) -> Option<i64> {
    read_sysfs(&format!("/sys/class/net/{}/mtu", iface)).and_then(|s| s.parse().ok())
}

/// List all non-loopback network interfaces.
fn list_interfaces() -> Vec<String> {
    let mut ifaces = Vec::new();

    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str == "lo" {
                continue;
            }
            ifaces.push(name_str);
        }
    }

    ifaces.sort();
    ifaces
}

// ── SCDynamicStore key → value builder ──────────────────────────

/// Look up a SCDynamicStore key and return a CF dict/array with real network data.
///
/// macOS key format: "State:/Network/Global/IPv4",
///                   "State:/Network/Interface/en0/IPv4",
///                   "State:/Network/Global/DNS", etc.
fn dynamic_store_lookup(key: &str) -> u64 {
    log::debug!("SCDynamicStoreCopyValue key=\"{}\"", key);

    // State:/Network/Global/IPv4 → primary interface + addresses
    if key.contains("/Network/Global/IPv4") || key.contains("/Network/Global/IPv6") {
        return build_global_ipv4_dict();
    }

    // State:/Network/Global/DNS → DNS servers
    if key.contains("/Network/Global/DNS")
        || key.contains("/Network/Service/") && key.ends_with("/DNS")
    {
        return build_dns_dict();
    }

    // State:/Network/Interface/{iface}/Link → link status
    if key.contains("/Network/Interface/") && key.ends_with("/Link") {
        if let Some(iface) = extract_interface_from_key(key) {
            return build_link_dict(&iface);
        }
    }

    // State:/Network/Interface/{iface}/IPv4 → addresses
    if key.contains("/Network/Interface/") && key.ends_with("/IPv4") {
        if let Some(iface) = extract_interface_from_key(key) {
            return build_interface_ipv4_dict(&iface);
        }
    }

    // State:/Network/Interface/{iface}/IPv6
    if key.contains("/Network/Interface/") && key.ends_with("/IPv6") {
        if let Some(iface) = extract_interface_from_key(key) {
            return build_interface_ipv6_dict(&iface);
        }
    }

    // State:/Network/Global/Proxies → empty dict (no proxies)
    if key.contains("/Network/Global/Proxies") || key.contains("/Proxies") {
        let dict = alloc_mutable_dict();
        dict_set(dict, alloc_cfstring("HTTPEnable"), alloc_cfnumber_i64(0));
        dict_set(dict, alloc_cfstring("HTTPSEnable"), alloc_cfnumber_i64(0));
        return dict;
    }

    // Setup:/Network/Global/IPv4 → same as State for our purposes
    if key.contains("Setup:/Network") {
        // Redirect to state-based lookup
        let state_key = key.replace("Setup:", "State:");
        return dynamic_store_lookup(&state_key);
    }

    0 // NULL — key not found
}

/// Extract interface name from key like "State:/Network/Interface/eth0/IPv4"
fn extract_interface_from_key(key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('/').collect();
    // Find "Interface" in parts and take the next element
    for (i, part) in parts.iter().enumerate() {
        if *part == "Interface" && i + 1 < parts.len() {
            return Some(parts[i + 1].to_string());
        }
    }
    None
}

/// Build State:/Network/Global/IPv4 dict.
fn build_global_ipv4_dict() -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    // Primary interface
    if let Some(iface) = get_primary_interface() {
        dict_set(
            dict,
            alloc_cfstring("PrimaryInterface"),
            alloc_cfstring(&iface),
        );
        dict_set(
            dict,
            alloc_cfstring("PrimaryService"),
            alloc_cfstring(&format!("burdettie-{}", iface)),
        );

        // Router (default gateway)
        if let Some(gw) = get_default_gateway() {
            dict_set(dict, alloc_cfstring("Router"), alloc_cfstring(&gw));
        }
    }

    dict
}

/// Build DNS dict.
fn build_dns_dict() -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    let servers = get_dns_servers();
    if !servers.is_empty() {
        let server_cfs: Vec<u64> = servers.iter().map(|s| alloc_cfstring(s)).collect();
        dict_set(
            dict,
            alloc_cfstring("ServerAddresses"),
            alloc_cfarray(&server_cfs),
        );
    }

    let domains = get_dns_search_domains();
    if !domains.is_empty() {
        let domain_cfs: Vec<u64> = domains.iter().map(|d| alloc_cfstring(d)).collect();
        dict_set(
            dict,
            alloc_cfstring("SearchDomains"),
            alloc_cfarray(&domain_cfs),
        );
    }

    dict
}

/// Build link status dict for an interface.
fn build_link_dict(iface: &str) -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }
    let active = is_link_active(iface);
    dict_set(
        dict,
        alloc_cfstring("Active"),
        alloc_cfnumber_i64(if active { 1 } else { 0 }),
    );
    dict
}

/// Build IPv4 dict for a specific interface.
fn build_interface_ipv4_dict(iface: &str) -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    let addrs = get_ipv4_addresses(iface);
    if !addrs.is_empty() {
        let addr_cfs: Vec<u64> = addrs.iter().map(|a| alloc_cfstring(a)).collect();
        dict_set(dict, alloc_cfstring("Addresses"), alloc_cfarray(&addr_cfs));
    }

    if let Some(mask) = get_subnet_mask(iface) {
        dict_set(
            dict,
            alloc_cfstring("SubnetMasks"),
            alloc_cfarray(&[alloc_cfstring(&mask)]),
        );
    }

    dict_set(dict, alloc_cfstring("InterfaceName"), alloc_cfstring(iface));

    dict
}

/// Build IPv6 dict for a specific interface.
fn build_interface_ipv6_dict(iface: &str) -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    let addrs = get_ipv6_addresses(iface);
    if !addrs.is_empty() {
        let addr_cfs: Vec<u64> = addrs.iter().map(|a| alloc_cfstring(a)).collect();
        dict_set(dict, alloc_cfstring("Addresses"), alloc_cfarray(&addr_cfs));
    }

    dict_set(dict, alloc_cfstring("InterfaceName"), alloc_cfstring(iface));

    dict
}

/// Build an SCNetworkInterface-like dict for a Linux interface.
fn build_interface_dict(iface: &str) -> u64 {
    let dict = alloc_mutable_dict();
    if dict == 0 {
        return 0;
    }

    dict_set(dict, alloc_cfstring("BSD Name"), alloc_cfstring(iface));
    dict_set(dict, alloc_cfstring("InterfaceName"), alloc_cfstring(iface));

    // Determine type
    let itype = if iface.starts_with("wl") || iface.starts_with("wlan") {
        "IEEE80211" // WiFi
    } else if iface.starts_with("eth")
        || iface.starts_with("en")
        || iface.starts_with("ens")
        || iface.starts_with("enp")
    {
        "Ethernet"
    } else if iface.starts_with("docker") || iface.starts_with("br-") || iface.starts_with("veth") {
        "Bridge"
    } else {
        "Other"
    };
    dict_set(dict, alloc_cfstring("Type"), alloc_cfstring(itype));
    dict_set(dict, alloc_cfstring("Hardware"), alloc_cfstring(itype));

    // Display name
    let display_name = match itype {
        "IEEE80211" => "Wi-Fi",
        "Ethernet" => "Ethernet",
        "Bridge" => "Bridge",
        _ => iface,
    };
    dict_set(
        dict,
        alloc_cfstring("UserDefinedName"),
        alloc_cfstring(display_name),
    );

    // MAC address
    if let Some(mac) = get_mac_address(iface) {
        dict_set(
            dict,
            alloc_cfstring("HardwareAddress"),
            alloc_cfstring(&mac),
        );
    }

    // MTU
    if let Some(mtu) = get_mtu(iface) {
        dict_set(dict, alloc_cfstring("MTU"), alloc_cfnumber_i64(mtu));
    }

    // Link active
    dict_set(
        dict,
        alloc_cfstring("Active"),
        alloc_cfnumber_i64(if is_link_active(iface) { 1 } else { 0 }),
    );

    dict
}

// ── SCNetworkReachability (real connectivity check) ─────────────

/// Perform a real reachability check for a hostname using getaddrinfo.
fn check_reachability(hostname: &str) -> u32 {
    // BurdettieOS DNS rewrite — redirect Apple domains to local service
    let resolve_hostname = match burdettie_domain_map::rewrite_hostname(hostname) {
        Some(rewritten) => rewritten,
        None => hostname.to_string(),
    };

    let Ok(cstr) = std::ffi::CString::new(resolve_hostname.as_str()) else {
        return 0;
    };

    let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    hints.ai_family = libc::AF_UNSPEC;
    hints.ai_socktype = libc::SOCK_STREAM;

    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let rc = unsafe { libc::getaddrinfo(cstr.as_ptr(), std::ptr::null(), &hints, &mut result) };

    if rc != 0 || result.is_null() {
        log::debug!("SCReachability: getaddrinfo failed for '{hostname}' → not reachable");
        return 0;
    }
    unsafe {
        libc::freeaddrinfo(result);
    }

    // DNS resolved — host is reachable
    log::trace!("SCReachability: '{hostname}' → reachable");
    K_SC_FLAG_REACHABLE
}

/// `SCNetworkReachabilityCreateWithName(alloc, name) -> SCNetworkReachabilityRef`
pub extern "C" fn shim_sc_reachability_create_with_name(_alloc: u64, name: u64) -> u64 {
    let hostname = if name != 0 {
        unsafe { std::ffi::CStr::from_ptr(name as *const std::ffi::c_char) }
            .to_str()
            .unwrap_or("localhost")
            .to_string()
    } else {
        "localhost".to_string()
    };

    let handle = next_handle();
    let target = ReachabilityTarget {
        hostname: Some(hostname.clone()),
        flags: 0,
    };
    let mut guard = reachability_store();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, target);
    log::trace!("SCNetworkReachabilityCreateWithName: host={hostname} → handle={handle:#x}");
    handle
}

/// `SCNetworkReachabilityCreateWithAddress(alloc, addr) -> SCNetworkReachabilityRef`
pub extern "C" fn shim_sc_reachability_create_with_address(_alloc: u64, addr: u64) -> u64 {
    // Extract IP from sockaddr if possible
    let hostname = if addr != 0 {
        let family = unsafe { *(addr as *const u8).add(1) }; // sa_family at offset 1 (macOS)
        if family == 2 {
            // AF_INET: extract 4-byte IP at offset 4
            let ip_bytes = unsafe { std::slice::from_raw_parts((addr as *const u8).add(4), 4) };
            Some(format!(
                "{}.{}.{}.{}",
                ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
            ))
        } else {
            None
        }
    } else {
        None
    };

    let handle = next_handle();
    let target = ReachabilityTarget { hostname, flags: 0 };
    let mut guard = reachability_store();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, target);
    log::trace!("SCNetworkReachabilityCreateWithAddress → handle={handle:#x}");
    handle
}

/// `SCNetworkReachabilityGetFlags(target, &flags) -> Boolean`
///
/// Real implementation: performs DNS lookup to check if host is reachable.
pub extern "C" fn shim_sc_reachability_get_flags(target: u64, flags: u64) -> u8 {
    let mut guard = reachability_store();
    let computed_flags = if let Some(store) = guard.as_mut() {
        if let Some(entry) = store.get_mut(&target) {
            let f = match &entry.hostname {
                Some(h) => check_reachability(h),
                None => K_SC_FLAG_REACHABLE, // No hostname → assume local
            };
            entry.flags = f;
            f
        } else {
            K_SC_FLAG_REACHABLE // Unknown handle, assume reachable for compat
        }
    } else {
        K_SC_FLAG_REACHABLE
    };

    if flags != 0 {
        unsafe {
            std::ptr::write(flags as *mut u32, computed_flags);
        }
    }
    1 // TRUE — function succeeded
}

/// `SCNetworkReachabilitySetCallback(target, callback, context) -> Boolean`
pub extern "C" fn shim_sc_reachability_set_callback(
    _target: u64,
    _callback: u64,
    _context: u64,
) -> u8 {
    1 // TRUE — no-op, we never fire callbacks
}

/// `SCNetworkReachabilityScheduleWithRunLoop(target, rl, mode) -> Boolean`
pub extern "C" fn shim_sc_reachability_schedule_with_run_loop(
    _target: u64,
    _rl: u64,
    _mode: u64,
) -> u8 {
    1 // TRUE
}

/// `SCNetworkReachabilityUnscheduleFromRunLoop(target, rl, mode) -> Boolean`
pub extern "C" fn shim_sc_reachability_unschedule_from_run_loop(
    _target: u64,
    _rl: u64,
    _mode: u64,
) -> u8 {
    1 // TRUE
}

/// `SCNetworkReachabilitySetDispatchQueue(target, queue) -> Boolean`
pub extern "C" fn shim_sc_reachability_set_dispatch_queue(_target: u64, _queue: u64) -> u8 {
    1 // TRUE
}

/// `SCNetworkReachabilityCreateWithAddressPair(alloc, localAddr, remoteAddr) → SCNetworkReachabilityRef`
pub extern "C" fn shim_sc_reachability_create_with_address_pair(
    _alloc: u64,
    _local_addr: u64,
    remote_addr: u64,
) -> u64 {
    // Extract hostname from remote address if AF_INET
    let hostname = if remote_addr != 0 {
        let family = unsafe { *(remote_addr as *const u8).add(1) };
        if family == 2 {
            let ip_bytes =
                unsafe { std::slice::from_raw_parts((remote_addr as *const u8).add(4), 4) };
            Some(format!(
                "{}.{}.{}.{}",
                ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
            ))
        } else {
            None
        }
    } else {
        None
    };

    let handle = next_handle();
    let target = ReachabilityTarget { hostname, flags: 0 };
    let mut guard = reachability_store();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, target);
    handle
}

// ── SCDynamicStore ──────────────────────────────────────────────

/// `SCDynamicStoreCreate(alloc, name, callback, ctx) -> SCDynamicStoreRef`
pub extern "C" fn shim_sc_dynamic_store_create(
    _alloc: u64,
    _name: u64,
    callback: u64,
    _ctx: u64,
) -> u64 {
    let handle = next_handle();
    let state = DynStoreState {
        notification_keys: Vec::new(),
        callback,
    };
    let mut guard = dynstore_state();
    guard.get_or_insert_with(HashMap::new).insert(handle, state);
    log::trace!("SCDynamicStoreCreate → handle={handle:#x}");
    handle
}

/// `SCDynamicStoreCopyValue(store, key) -> CFPropertyListRef`
///
/// Returns real network state from Linux sysfs/procfs for known keys.
pub extern "C" fn shim_sc_dynamic_store_copy_value(_store: u64, key: u64) -> u64 {
    if key == 0 {
        return 0;
    }

    let key_str = match extract_key_string(key) {
        Some(k) => k,
        None => {
            // Try as C string
            let cstr = unsafe { CStr::from_ptr(key as *const _) };
            match cstr.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => return 0,
            }
        }
    };

    dynamic_store_lookup(&key_str)
}

/// `SCDynamicStoreCopyMultiple(store, keys, patterns) -> CFDictionaryRef`
pub extern "C" fn shim_sc_dynamic_store_copy_multiple(
    _store: u64,
    _keys: u64,
    _patterns: u64,
) -> u64 {
    // Return an empty dict — full key enumeration not yet implemented
    alloc_mutable_dict()
}

/// `SCDynamicStoreSetValue(store, key, value) -> Boolean`
pub extern "C" fn shim_sc_dynamic_store_set_value(_store: u64, _key: u64, _value: u64) -> u8 {
    1 // TRUE — accept but ignore writes
}

/// `SCDynamicStoreSetNotificationKeys(store, keys, patterns) -> Boolean`
pub extern "C" fn shim_sc_dynamic_store_set_notification_keys(
    _store: u64,
    _keys: u64,
    _patterns: u64,
) -> u8 {
    1 // TRUE — we accept the registration but don't fire callbacks yet
}

/// `SCDynamicStoreSetDispatchQueue(store, queue) -> Boolean`
pub extern "C" fn shim_sc_dynamic_store_set_dispatch_queue(_store: u64, _queue: u64) -> u8 {
    1 // TRUE
}

/// `SCDynamicStoreCopyKeyList(store, pattern) -> CFArrayRef`
///
/// Returns an array of known dynamic store keys matching the pattern.
pub extern "C" fn shim_sc_dynamic_store_copy_key_list(_store: u64, _pattern: u64) -> u64 {
    // Return a list of keys we know about
    let mut keys = Vec::new();

    if let Some(iface) = get_primary_interface() {
        keys.push(alloc_cfstring(&format!(
            "State:/Network/Interface/{}/IPv4",
            iface
        )));
        keys.push(alloc_cfstring(&format!(
            "State:/Network/Interface/{}/IPv6",
            iface
        )));
        keys.push(alloc_cfstring(&format!(
            "State:/Network/Interface/{}/Link",
            iface
        )));
    }
    keys.push(alloc_cfstring("State:/Network/Global/IPv4"));
    keys.push(alloc_cfstring("State:/Network/Global/DNS"));
    keys.push(alloc_cfstring("State:/Network/Global/Proxies"));

    alloc_cfarray(&keys)
}

/// `SCDynamicStoreCreateRunLoopSource(alloc, store, order) -> CFRunLoopSourceRef`
pub extern "C" fn shim_sc_dynamic_store_create_run_loop_source(
    _alloc: u64,
    _store: u64,
    _order: i64,
) -> u64 {
    // Return a non-zero sentinel — the run loop source won't actually fire
    // but apps check for non-NULL to confirm registration succeeded
    next_handle()
}

/// `SCDynamicStoreNotifyValue(store, key) -> Boolean`
pub extern "C" fn shim_sc_dynamic_store_notify_value(_store: u64, _key: u64) -> u8 {
    1 // TRUE
}

// ── SCPreferences (in-memory key-value store) ──────────────────

/// `SCPreferencesCreate(alloc, name, prefs_id) -> SCPreferencesRef`
pub extern "C" fn shim_sc_preferences_create(_alloc: u64, _name: u64, _prefs_id: u64) -> u64 {
    let handle = next_handle();
    let mut guard = prefs_store();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, HashMap::new());
    handle
}

/// `SCPreferencesGetValue(prefs, key) -> CFPropertyListRef`
pub extern "C" fn shim_sc_preferences_get_value(prefs: u64, key: u64) -> u64 {
    let Some(key_str) = extract_key_string(key) else {
        return 0;
    };

    let guard = prefs_store();
    if let Some(store) = guard.as_ref() {
        if let Some(prefs_map) = store.get(&prefs) {
            if let Some(&val) = prefs_map.get(&key_str) {
                return val;
            }
        }
    }
    0 // NULL — not found
}

/// `SCPreferencesSetValue(prefs, key, value) -> Boolean`
pub extern "C" fn shim_sc_preferences_set_value(prefs: u64, key: u64, value: u64) -> u8 {
    let Some(key_str) = extract_key_string(key) else {
        return 0;
    };

    let mut guard = prefs_store();
    if let Some(store) = guard.as_mut() {
        if let Some(prefs_map) = store.get_mut(&prefs) {
            prefs_map.insert(key_str, value);
            return 1;
        }
    }
    1 // TRUE — silently succeed even if handle unknown
}

/// `SCPreferencesCommitChanges(prefs) -> Boolean`
pub extern "C" fn shim_sc_preferences_commit_changes(_prefs: u64) -> u8 {
    1 // TRUE — in-memory only
}

/// `SCPreferencesApplyChanges(prefs) -> Boolean`
pub extern "C" fn shim_sc_preferences_apply_changes(_prefs: u64) -> u8 {
    1 // TRUE
}

/// `SCPreferencesPathGetValue(prefs, path) -> CFDictionaryRef`
pub extern "C" fn shim_sc_preferences_path_get_value(_prefs: u64, _path: u64) -> u64 {
    0 // NULL
}

/// `SCPreferencesLock(prefs, wait) -> Boolean`
pub extern "C" fn shim_sc_preferences_lock(_prefs: u64, _wait: u8) -> u8 {
    1 // TRUE
}

/// `SCPreferencesUnlock(prefs) -> Boolean`
pub extern "C" fn shim_sc_preferences_unlock(_prefs: u64) -> u8 {
    1 // TRUE
}

// ── SCNetworkConfiguration (real interface data) ────────────────

/// `SCNetworkInterfaceCopyAll() -> CFArrayRef`
///
/// Returns a CFArray of interface dicts populated from /sys/class/net/.
pub extern "C" fn shim_sc_network_interface_copy_all() -> u64 {
    let ifaces = list_interfaces();
    if ifaces.is_empty() {
        return alloc_cfarray(&[]);
    }

    let entries: Vec<u64> = ifaces.iter().map(|i| build_interface_dict(i)).collect();
    alloc_cfarray(&entries)
}

/// `SCNetworkServiceCopyAll(prefs) -> CFArrayRef`
pub extern "C" fn shim_sc_network_service_copy_all(_prefs: u64) -> u64 {
    alloc_cfarray(&[]) // empty array
}

/// `SCNetworkInterfaceGetBSDName(interface) -> CFStringRef`
pub extern "C" fn shim_sc_network_interface_get_bsd_name(iface_dict: u64) -> u64 {
    // The interface is represented as a dict; read "BSD Name" key
    if iface_dict == 0 {
        return 0;
    }
    dict_get_value(iface_dict, "BSD Name")
}

/// `SCNetworkInterfaceGetInterfaceType(interface) -> CFStringRef`
pub extern "C" fn shim_sc_network_interface_get_interface_type(iface_dict: u64) -> u64 {
    if iface_dict == 0 {
        return 0;
    }
    dict_get_value(iface_dict, "Type")
}

/// `SCNetworkInterfaceGetHardwareAddressString(interface) -> CFStringRef`
pub extern "C" fn shim_sc_network_interface_get_hw_addr(iface_dict: u64) -> u64 {
    if iface_dict == 0 {
        return 0;
    }
    dict_get_value(iface_dict, "HardwareAddress")
}

/// `SCNetworkInterfaceGetLocalizedDisplayName(interface) -> CFStringRef`
pub extern "C" fn shim_sc_network_interface_get_display_name(iface_dict: u64) -> u64 {
    if iface_dict == 0 {
        return 0;
    }
    dict_get_value(iface_dict, "UserDefinedName")
}

/// `SCNetworkInterfaceGetConfiguration(interface) -> CFDictionaryRef`
pub extern "C" fn shim_sc_network_interface_get_configuration(_iface_dict: u64) -> u64 {
    0 // NULL
}

/// Helper: read a value from a CF mutable dict by key string.
fn dict_get_value(dict: u64, target_key: &str) -> u64 {
    if dict == 0 {
        return 0;
    }
    unsafe {
        let base = dict as *const u8;
        let count = *(base.add(8) as *const u64) as usize;
        let keys_ptr = *(base.add(16) as *const u64) as *const u64;
        let vals_ptr = *(base.add(24) as *const u64) as *const u64;

        if keys_ptr.is_null() || vals_ptr.is_null() {
            return 0;
        }

        for i in 0..count {
            let key = *keys_ptr.add(i);
            if let Some(key_str) = extract_key_string(key) {
                if key_str == target_key {
                    return *vals_ptr.add(i);
                }
            }
        }
    }
    0
}

// ── Error handling ──────────────────────────────────────────────

/// `SCError() -> int`
pub extern "C" fn shim_sc_error() -> i32 {
    0 // kSCStatusOK
}

/// `SCErrorString(status) -> const char *`
pub extern "C" fn shim_sc_error_string(_status: i32) -> u64 {
    static SC_OK_MSG: &[u8] = b"Success\0";
    SC_OK_MSG.as_ptr() as u64
}

/// `SCCopyLastError() -> CFErrorRef`
pub extern "C" fn shim_sc_copy_last_error() -> u64 {
    0 // NULL — no error
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Reachability tests ──────────────────────────────────────

    #[test]
    fn test_reachability_create_returns_handle() {
        let h1 = shim_sc_reachability_create_with_name(0, 0);
        assert_ne!(h1, 0);
        let h2 = shim_sc_reachability_create_with_address(0, 0);
        assert_ne!(h2, 0);
        assert_ne!(h1, h2, "each call should return a unique handle");
    }

    #[test]
    fn test_reachability_flags_checks_real_host() {
        let hostname = std::ffi::CString::new("localhost").unwrap();
        let handle = shim_sc_reachability_create_with_name(0, hostname.as_ptr() as u64);
        assert_ne!(handle, 0);

        let mut flags: u32 = 0;
        let ret = shim_sc_reachability_get_flags(handle, &mut flags as *mut u32 as u64);
        assert_eq!(ret, 1, "GetFlags should return TRUE");
        assert_ne!(
            flags & K_SC_FLAG_REACHABLE,
            0,
            "localhost should be reachable"
        );
    }

    #[test]
    fn test_reachability_create_with_address_pair() {
        let h = shim_sc_reachability_create_with_address_pair(0, 0, 0);
        assert_ne!(h, 0, "CreateWithAddressPair should return a handle");
    }

    #[test]
    fn test_reachability_schedule_succeeds() {
        let ret = shim_sc_reachability_schedule_with_run_loop(1, 0, 0);
        assert_eq!(ret, 1, "ScheduleWithRunLoop should return TRUE");
    }

    // ── SCDynamicStore tests ────────────────────────────────────

    #[test]
    fn test_dynamic_store_create_returns_handle() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        assert_ne!(store, 0);
    }

    #[test]
    fn test_dynamic_store_copy_value_null_key() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let val = shim_sc_dynamic_store_copy_value(store, 0);
        assert_eq!(val, 0, "NULL key should return NULL");
    }

    #[test]
    fn test_dynamic_store_copy_value_global_ipv4() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let key = alloc_cfstring("State:/Network/Global/IPv4");
        let dict = shim_sc_dynamic_store_copy_value(store, key);
        assert_ne!(dict, 0, "Global/IPv4 should return a non-NULL dict");
    }

    #[test]
    fn test_dynamic_store_copy_value_dns() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let key = alloc_cfstring("State:/Network/Global/DNS");
        let dict = shim_sc_dynamic_store_copy_value(store, key);
        assert_ne!(dict, 0, "Global/DNS should return a non-NULL dict");
    }

    #[test]
    fn test_dynamic_store_copy_value_proxies() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let key = alloc_cfstring("State:/Network/Global/Proxies");
        let dict = shim_sc_dynamic_store_copy_value(store, key);
        assert_ne!(dict, 0, "Global/Proxies should return a non-NULL dict");
    }

    #[test]
    fn test_dynamic_store_copy_value_unknown_key() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let key = alloc_cfstring("State:/No/Such/Key");
        let val = shim_sc_dynamic_store_copy_value(store, key);
        assert_eq!(val, 0, "Unknown key should return NULL");
    }

    #[test]
    fn test_dynamic_store_copy_key_list_returns_array() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let arr = shim_sc_dynamic_store_copy_key_list(store, 0);
        assert_ne!(arr, 0, "CopyKeyList should return a non-NULL array");
    }

    #[test]
    fn test_dynamic_store_create_run_loop_source_nonzero() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let src = shim_sc_dynamic_store_create_run_loop_source(0, store, 0);
        assert_ne!(src, 0, "RunLoopSource should return non-zero");
    }

    #[test]
    fn test_dynamic_store_notify_succeeds() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let ret = shim_sc_dynamic_store_notify_value(store, 0);
        assert_eq!(ret, 1, "NotifyValue should return TRUE");
    }

    #[test]
    fn test_dynamic_store_copy_multiple_returns_dict() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        let result = shim_sc_dynamic_store_copy_multiple(store, 0, 0);
        assert_ne!(result, 0, "CopyMultiple should return a dict (not NULL)");
    }

    // ── SCPreferences tests ─────────────────────────────────────

    #[test]
    fn test_preferences_create_returns_handle() {
        let h = shim_sc_preferences_create(0, 0, 0);
        assert_ne!(h, 0);
    }

    #[test]
    fn test_preferences_set_and_get() {
        let prefs = shim_sc_preferences_create(0, 0, 0);
        let key = alloc_cfstring("TestKey");
        let val = alloc_cfstring("TestValue");

        let ret = shim_sc_preferences_set_value(prefs, key, val);
        assert_eq!(ret, 1, "SetValue should return TRUE");

        let got = shim_sc_preferences_get_value(prefs, key);
        assert_eq!(got, val, "GetValue should return the set value");
    }

    #[test]
    fn test_preferences_get_unknown_returns_null() {
        let prefs = shim_sc_preferences_create(0, 0, 0);
        let key = alloc_cfstring("NoSuchKey");
        let val = shim_sc_preferences_get_value(prefs, key);
        assert_eq!(val, 0, "Unknown key should return NULL");
    }

    #[test]
    fn test_preferences_lock_unlock() {
        let prefs = shim_sc_preferences_create(0, 0, 0);
        assert_eq!(shim_sc_preferences_lock(prefs, 0), 1);
        assert_eq!(shim_sc_preferences_unlock(prefs), 1);
    }

    // ── SCNetworkConfiguration tests ────────────────────────────

    #[test]
    fn test_network_interface_copy_all_returns_array() {
        let arr = shim_sc_network_interface_copy_all();
        // Should return a non-NULL CFArray (even if empty on macOS host)
        assert_ne!(arr, 0, "SCNetworkInterfaceCopyAll should return non-NULL");
    }

    #[test]
    fn test_network_service_copy_all_returns_array() {
        let arr = shim_sc_network_service_copy_all(0);
        assert_ne!(
            arr, 0,
            "SCNetworkServiceCopyAll should return non-NULL (empty array)"
        );
    }

    // ── Error handling tests ────────────────────────────────────

    #[test]
    fn test_sc_error_returns_zero() {
        assert_eq!(shim_sc_error(), 0, "SCError should return 0 (kSCStatusOK)");
    }

    #[test]
    fn test_sc_error_string_returns_nonnull() {
        let s = shim_sc_error_string(0);
        assert_ne!(s, 0, "SCErrorString should return a valid string");
    }

    #[test]
    fn test_sc_copy_last_error_returns_null() {
        let e = shim_sc_copy_last_error();
        assert_eq!(e, 0, "SCCopyLastError should return NULL when no error");
    }

    // ── Data symbol tests ───────────────────────────────────────

    #[test]
    fn test_data_symbols_stable() {
        assert_eq!(K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE, 2);
        assert_eq!(KSCDYNAMIC_STORE_DOMAIN_STATE[..5], *b"State");
        assert_eq!(KSCCOMP_NETWORK[..7], *b"Network");
        assert_eq!(KSCENT_NET_IPV4[..4], *b"IPv4");
        assert_eq!(KSCENT_NET_DNS[..3], *b"DNS");
    }

    // ── CF helper tests ─────────────────────────────────────────

    #[test]
    fn test_alloc_cfstring_roundtrip() {
        let s = "hello SC";
        let ptr = alloc_cfstring(s);
        assert_ne!(ptr, 0);
        let result = extract_key_string(ptr);
        assert_eq!(result.as_deref(), Some(s));
    }

    #[test]
    fn test_alloc_cfarray_count() {
        let items = [
            alloc_cfstring("a"),
            alloc_cfstring("b"),
            alloc_cfstring("c"),
        ];
        let arr = alloc_cfarray(&items);
        assert_ne!(arr, 0);
        let count = unsafe { *((arr as usize + 8) as *const u64) };
        assert_eq!(count, 3);
    }

    #[test]
    fn test_dict_set_and_get() {
        let dict = alloc_mutable_dict();
        assert_ne!(dict, 0);

        let key = alloc_cfstring("mykey");
        let val = alloc_cfstring("myval");
        dict_set(dict, key, val);

        let got = dict_get_value(dict, "mykey");
        assert_eq!(got, val, "should retrieve the value we set");
    }

    // ── Network helpers tests ───────────────────────────────────

    #[test]
    fn test_get_dns_servers_reads_resolv_conf() {
        let servers = get_dns_servers();
        // Systems without resolver configuration may have no DNS servers.
        // Any returned address must be a complete, nonempty token.
        for server in servers {
            assert!(!server.is_empty());
            assert!(!server.chars().any(char::is_whitespace));
        }
    }

    #[test]
    fn test_list_interfaces_no_loopback() {
        let ifaces = list_interfaces();
        for iface in &ifaces {
            assert_ne!(iface.as_str(), "lo", "should not include loopback");
        }
    }

    #[test]
    fn test_build_global_ipv4_dict_nonzero() {
        let dict = build_global_ipv4_dict();
        assert_ne!(dict, 0);
    }

    #[test]
    fn test_build_dns_dict_nonzero() {
        let dict = build_dns_dict();
        assert_ne!(dict, 0);
    }
}
