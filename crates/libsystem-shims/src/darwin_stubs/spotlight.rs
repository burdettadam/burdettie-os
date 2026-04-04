//! Spotlight / Metadata framework stubs.
//!
//! MDQuery/MDItem functions interact with the mds Spotlight daemon via
//! Mach ports. Forwarding these to the real CoreServices implementation
//! inside our interpreter causes IPC crashes. Instead we return safe
//! sentinel values so the guest binary exits cleanly.
//!
//! kMD* string constants (e.g. kMDItemPath) still resolve to their
//! real addresses via StubResolver (CoreServices is preloaded).

use crate::table::ShimTable;

pub struct SpotlightStubs;

impl SpotlightStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            // ── MDQuery creation: return NULL so callers detect failure ──
            ("_MDQueryCreate",              shim_md_null as *const () as u64, "MDQueryCreate (stub → NULL)"),
            ("_MDQueryCreateQueryString",   shim_md_null as *const () as u64, "MDQueryCreateQueryString (stub → NULL)"),
            ("_MDQueryBatchingParams",      shim_md_null as *const () as u64, "MDQueryBatchingParams (stub → NULL)"),
            // ── MDQuery operations (all take an MDQueryRef that will be NULL) ──
            ("_MDQuerySetMaxCount",         shim_md_void2 as *const () as u64, "MDQuerySetMaxCount (stub → void)"),
            ("_MDQuerySetSearchScope",      shim_md_void2 as *const () as u64, "MDQuerySetSearchScope (stub → void)"),
            ("_MDQuerySetAttributeFilter",  shim_md_void2 as *const () as u64, "MDQuerySetAttributeFilter (stub → void)"),
            ("_MDQuerySetSortOrder",        shim_md_void2 as *const () as u64, "MDQuerySetSortOrder (stub → void)"),
            ("_MDQuerySetSortComparator",   shim_md_void3 as *const () as u64, "MDQuerySetSortComparator (stub → void)"),
            ("_MDQueryExecute",             shim_md_false as *const () as u64, "MDQueryExecute (stub → false)"),
            ("_MDQueryStop",                shim_md_void1 as *const () as u64, "MDQueryStop (stub → void)"),
            ("_MDQueryEnable",              shim_md_void1 as *const () as u64, "MDQueryEnable (stub → void)"),
            ("_MDQueryDisable",             shim_md_void1 as *const () as u64, "MDQueryDisable (stub → void)"),
            ("_MDQueryGetResultCount",      shim_md_zero as *const () as u64, "MDQueryGetResultCount (stub → 0)"),
            ("_MDQueryGetResultAtIndex",    shim_md_null as *const () as u64, "MDQueryGetResultAtIndex (stub → NULL)"),
            ("_MDQueryCopyResultAtIndex",   shim_md_null as *const () as u64, "MDQueryCopyResultAtIndex (stub → NULL)"),
            ("_MDQueryGetIndexOfResult",    shim_md_zero as *const () as u64, "MDQueryGetIndexOfResult (stub → 0)"),
            ("_MDQueryGetAttributeValueOfResultAtIndex", shim_md_null as *const () as u64, "MDQueryGetAttributeValueOfResultAtIndex (stub → NULL)"),
            ("_MDQueryCopyValuesOfAttribute", shim_md_null as *const () as u64, "MDQueryCopyValuesOfAttribute (stub → NULL)"),
            ("_MDQueryGetTypeID",           shim_md_zero as *const () as u64, "MDQueryGetTypeID (stub → 0)"),
            // ── MDItem — always NULL, callers should check before use ──
            ("_MDItemCreate",               shim_md_null as *const () as u64, "MDItemCreate (stub → NULL)"),
            ("_MDItemCopyAttribute",        shim_md_null as *const () as u64, "MDItemCopyAttribute (stub → NULL)"),
            ("_MDItemCopyAttributes",       shim_md_null as *const () as u64, "MDItemCopyAttributes (stub → NULL)"),
            ("_MDItemGetTypeID",            shim_md_zero as *const () as u64, "MDItemGetTypeID (stub → 0)"),
            // ── Private MDQuery SPI used by /usr/bin/mdfind ──
            ("__MDQueryCreateQueryString",  shim_md_null as *const () as u64, "_MDQueryCreateQueryString SPI (stub → NULL)"),
            ("__MDQueryExecute",            shim_md_false as *const () as u64, "_MDQueryExecute SPI (stub → false)"),
            ("__MDQueryStop",               shim_md_void1 as *const () as u64, "_MDQueryStop SPI (stub → void)"),
            ("__MDQueryGetResultCount",     shim_md_zero as *const () as u64, "_MDQueryGetResultCount SPI (stub → 0)"),
            ("__MDQueryGetResultAtIndex",   shim_md_null as *const () as u64, "_MDQueryGetResultAtIndex SPI (stub → NULL)"),
            ("__MDItemCreate",              shim_md_null as *const () as u64, "_MDItemCreate SPI (stub → NULL)"),
            ("__MDItemCopyAttribute",       shim_md_null as *const () as u64, "_MDItemCopyAttribute SPI (stub → NULL)"),
        );
        log::debug!("SpotlightStubs: registered Spotlight/Metadata stubs");
    }
}

/// Returns 0 (NULL for pointer types).
pub extern "C" fn shim_md_null(_a0: u64, _a1: u64, _a2: u64, _a3: u64) -> u64 { 0 }
/// Returns 0 (CFIndex / count).
pub extern "C" fn shim_md_zero(_a0: u64, _a1: u64) -> u64 { 0 }
/// Returns false (Boolean).
pub extern "C" fn shim_md_false(_a0: u64, _a1: u64) -> u8 { 0 }
/// No-op, one argument.
pub extern "C" fn shim_md_void1(_a0: u64) {}
/// No-op, two arguments.
pub extern "C" fn shim_md_void2(_a0: u64, _a1: u64) {}
/// No-op, three arguments.
pub extern "C" fn shim_md_void3(_a0: u64, _a1: u64, _a2: u64) {}
