//! CoreData stub classes.
//!
//! Provides minimal ObjC class shims for NSManagedObjectContext,
//! NSPersistentStoreCoordinator, NSFetchRequest, NSManagedObject,
//! NSEntityDescription, and NSManagedObjectModel.  All methods return
//! nil / 0 / empty-error so CoreData-using code can instantiate these
//! classes without crashing on unrecognized selectors.

use crate::nsobj;

// ── Instance sizes ────────────────────────────────────────────────────────────
pub const NSMANAGEDOBJECTCONTEXT_SIZE:     usize = 32;
pub const NSPERSISTENTSTORECOORDINATOR_SIZE: usize = 24;
pub const NSFETCHREQUEST_SIZE:             usize = 40;
pub const NSMANAGEDOBJECT_SIZE:            usize = 24;
pub const NSENTITYDESCRIPTION_SIZE:        usize = 24;
pub const NSMANAGEDOBJECTMODEL_SIZE:       usize = 24;

// ── NSManagedObjectContext ────────────────────────────────────────────────────
// Layout: [ISA:8][coordinator:8][concurrencyType:8][reserved:8]

pub extern "C" fn shim_nsmoc_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMANAGEDOBJECTCONTEXT_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_nsmoc_init(obj: u64, _cmd: u64) -> u64 { obj }

pub extern "C" fn shim_nsmoc_init_with_concurrency_type(obj: u64, _cmd: u64, concurrency_type: u64) -> u64 {
    if obj != 0 {
        unsafe { *((obj as usize + 16) as *mut u64) = concurrency_type; }
    }
    obj
}

pub extern "C" fn shim_nsmoc_new(cls: u64, cmd: u64) -> u64 {
    shim_nsmoc_init(shim_nsmoc_alloc(cls, cmd), cmd)
}

/// `performBlock:` — no-op (block would need to be invoked; stub silently drops).
pub extern "C" fn shim_nsmoc_perform_block(_obj: u64, _cmd: u64, _block: u64) {}

/// `save:` — always succeeds (sets *error = nil), returns YES.
pub extern "C" fn shim_nsmoc_save(obj: u64, _cmd: u64, error_out: u64) -> u64 {
    if error_out != 0 {
        unsafe { *(error_out as *mut u64) = 0; }
    }
    if obj == 0 { return 0; }
    1
}

/// `executeFetchRequest:error:` — returns empty NSArray.
pub extern "C" fn shim_nsmoc_execute_fetch_request(_obj: u64, _cmd: u64, _request: u64, error_out: u64) -> u64 {
    if error_out != 0 {
        unsafe { *(error_out as *mut u64) = 0; }
    }
    crate::nsarray::make_nsarray(&[])
}

pub extern "C" fn shim_nsmoc_persistent_store_coordinator(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmoc_set_persistent_store_coordinator(_obj: u64, _cmd: u64, _coord: u64) {}
pub extern "C" fn shim_nsmoc_merge_changes_from_context_did_save(_obj: u64, _cmd: u64, _notif: u64) {}
pub extern "C" fn shim_nsmoc_undo_manager(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmoc_has_changes(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmoc_reset(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_nsmoc_rollback(_obj: u64, _cmd: u64) {}

// ── NSPersistentStoreCoordinator ──────────────────────────────────────────────

pub extern "C" fn shim_nspsc_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSPERSISTENTSTORECOORDINATOR_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_nspsc_init_with_model(obj: u64, _cmd: u64, _model: u64) -> u64 { obj }
pub extern "C" fn shim_nspsc_new(cls: u64, cmd: u64) -> u64 { shim_nspsc_alloc(cls, cmd) }

/// `addPersistentStoreWithType:configuration:URL:options:error:` → returns nil, sets *error = nil.
pub extern "C" fn shim_nspsc_add_store(
    _obj: u64, _cmd: u64,
    _store_type: u64, _config: u64, _url: u64, _options: u64,
    error_out: u64,
) -> u64 {
    if error_out != 0 {
        unsafe { *(error_out as *mut u64) = 0; }
    }
    0
}

pub extern "C" fn shim_nspsc_managed_object_model(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nspsc_persistent_stores(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

// ── NSFetchRequest ────────────────────────────────────────────────────────────
// Layout: [ISA:8][entityName:8][predicate:8][sortDescs:8][fetchLimit:8]

const NSFR_ENTITY_OFF:      usize = 8;
const NSFR_PREDICATE_OFF:   usize = 16;
const NSFR_SORTDESCS_OFF:   usize = 24;
const NSFR_FETCHLIMIT_OFF:  usize = 32;

pub extern "C" fn shim_nsfr_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSFETCHREQUEST_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_nsfr_init_with_entity_name(obj: u64, _cmd: u64, entity_name: u64) -> u64 {
    if obj != 0 {
        unsafe { *((obj as usize + NSFR_ENTITY_OFF) as *mut u64) = entity_name; }
    }
    obj
}

pub extern "C" fn shim_nsfr_fetch_request_with_entity_name(cls: u64, cmd: u64, entity_name: u64) -> u64 {
    let obj = shim_nsfr_alloc(cls, cmd);
    shim_nsfr_init_with_entity_name(obj, cmd, entity_name)
}

pub extern "C" fn shim_nsfr_entity_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + NSFR_ENTITY_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_predicate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + NSFR_PREDICATE_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_set_predicate(obj: u64, _cmd: u64, pred: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + NSFR_PREDICATE_OFF) as *mut u64) = pred; }
}

pub extern "C" fn shim_nsfr_sort_descriptors(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + NSFR_SORTDESCS_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_set_sort_descriptors(obj: u64, _cmd: u64, descs: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + NSFR_SORTDESCS_OFF) as *mut u64) = descs; }
}

pub extern "C" fn shim_nsfr_fetch_limit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + NSFR_FETCHLIMIT_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_set_fetch_limit(obj: u64, _cmd: u64, limit: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + NSFR_FETCHLIMIT_OFF) as *mut u64) = limit; }
}

// ── NSManagedObject ────────────────────────────────────────────────────────────
// Layout: [ISA:8][entity:8][context:8]

pub extern "C" fn shim_nsmo_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMANAGEDOBJECT_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_nsmo_init_with_entity_context(obj: u64, _cmd: u64, entity: u64, context: u64) -> u64 {
    if obj != 0 {
        unsafe {
            *((obj as usize + 8) as *mut u64) = entity;
            *((obj as usize + 16) as *mut u64) = context;
        }
    }
    obj
}

pub extern "C" fn shim_nsmo_value_for_key(_obj: u64, _cmd: u64, _key: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmo_set_value_for_key(_obj: u64, _cmd: u64, _value: u64, _key: u64) {}
pub extern "C" fn shim_nsmo_entity(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmo_managed_object_context(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmo_is_deleted(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmo_is_inserted(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsmo_is_updated(_obj: u64, _cmd: u64) -> u64 { 0 }

// ── NSEntityDescription ───────────────────────────────────────────────────────

pub extern "C" fn shim_nsed_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSENTITYDESCRIPTION_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_nsed_init(obj: u64, _cmd: u64) -> u64 { obj }

/// `+[NSEntityDescription entityForName:inManagedObjectContext:]` → nil
pub extern "C" fn shim_nsed_entity_for_name(_cls: u64, _cmd: u64, _name: u64, _ctx: u64) -> u64 { 0 }

pub extern "C" fn shim_nsed_name(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_nsed_properties(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}
pub extern "C" fn shim_nsed_managed_object_class_name(_obj: u64, _cmd: u64) -> u64 { 0 }

// ── NSManagedObjectModel ──────────────────────────────────────────────────────

pub extern "C" fn shim_nsmom_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMANAGEDOBJECTMODEL_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_nsmom_init(obj: u64, _cmd: u64) -> u64 { obj }
pub extern "C" fn shim_nsmom_init_with_contents_of_url(obj: u64, _cmd: u64, _url: u64) -> u64 { obj }

/// `+[NSManagedObjectModel mergedModelFromBundles:]` → nil
pub extern "C" fn shim_nsmom_merged_model_from_bundles(_cls: u64, _cmd: u64, _bundles: u64) -> u64 { 0 }

pub extern "C" fn shim_nsmom_entities(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

pub extern "C" fn shim_nsmom_entity_versions_by_name(_obj: u64, _cmd: u64) -> u64 { 0 }
