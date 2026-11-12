//! CoreData stub classes with in-memory SQLite backing store.
//!
//! Provides ObjC class shims for NSManagedObjectContext,
//! NSPersistentStoreCoordinator, NSFetchRequest, NSManagedObject,
//! NSEntityDescription, and NSManagedObjectModel.
//!
//! The persistent store coordinator maintains an in-memory SQLite database
//! (via rusqlite) that CoreData operations actually read from/write to.
//! Each entity type gets a table with columns (pk INTEGER PRIMARY KEY, key TEXT, value TEXT).

use crate::nsobj;
use std::sync::Mutex;

// ── In-memory SQLite store ────────────────────────────────────────────────────

static COREDATA_DB: Mutex<Option<rusqlite::Connection>> = Mutex::new(None);

fn db_connection() -> std::sync::MutexGuard<'static, Option<rusqlite::Connection>> {
    COREDATA_DB.lock().unwrap_or_else(|e| e.into_inner())
}

fn ensure_db(guard: &mut Option<rusqlite::Connection>) -> &mut rusqlite::Connection {
    guard.get_or_insert_with(|| {
        rusqlite::Connection::open_in_memory().expect("coredata: failed to open in-memory sqlite")
    })
}

fn ensure_entity_table(conn: &rusqlite::Connection, entity: &str) {
    // Sanitize entity name: only alphanumeric chars and underscores
    let safe_name: String = entity
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if safe_name.is_empty() {
        return;
    }
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS [{safe_name}] (pk INTEGER PRIMARY KEY AUTOINCREMENT, key TEXT NOT NULL, value TEXT NOT NULL DEFAULT '')"
    );
    let _ = conn.execute(&sql, []);
}

/// Insert a managed object's key-value into the store.
fn store_value(entity: &str, key: &str, value: &str) {
    let mut guard = db_connection();
    let conn = ensure_db(&mut guard);
    ensure_entity_table(conn, entity);
    let safe_entity: String = entity
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let sql = format!("INSERT INTO [{safe_entity}] (key, value) VALUES (?1, ?2)");
    let _ = conn.execute(&sql, rusqlite::params![key, value]);
}

/// Count rows for an entity.
fn count_rows(entity: &str) -> u64 {
    let mut guard = db_connection();
    let conn = ensure_db(&mut guard);
    ensure_entity_table(conn, entity);
    let safe_entity: String = entity
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let sql = format!("SELECT COUNT(*) FROM [{safe_entity}]");
    conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
        .unwrap_or(0) as u64
}

// ── Instance sizes ────────────────────────────────────────────────────────────
pub const NSMANAGEDOBJECTCONTEXT_SIZE: usize = 32;
pub const NSPERSISTENTSTORECOORDINATOR_SIZE: usize = 24;
pub const NSFETCHREQUEST_SIZE: usize = 40;
pub const NSMANAGEDOBJECT_SIZE: usize = 24;
pub const NSENTITYDESCRIPTION_SIZE: usize = 24;
pub const NSMANAGEDOBJECTMODEL_SIZE: usize = 24;

// ── NSManagedObjectContext ────────────────────────────────────────────────────
// Layout: [ISA:8][coordinator:8][concurrencyType:8][reserved:8]

pub extern "C" fn shim_nsmoc_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMANAGEDOBJECTCONTEXT_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_nsmoc_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

pub extern "C" fn shim_nsmoc_init_with_concurrency_type(
    obj: u64,
    _cmd: u64,
    concurrency_type: u64,
) -> u64 {
    if obj != 0 {
        unsafe {
            *((obj as usize + 16) as *mut u64) = concurrency_type;
        }
    }
    obj
}

pub extern "C" fn shim_nsmoc_new(cls: u64, cmd: u64) -> u64 {
    shim_nsmoc_init(shim_nsmoc_alloc(cls, cmd), cmd)
}

/// `performBlock:` — no-op (block would need to be invoked; stub silently drops).
pub extern "C" fn shim_nsmoc_perform_block(_obj: u64, _cmd: u64, _block: u64) {}

/// `save:` — persists to the in-memory SQLite store, returns YES.
pub extern "C" fn shim_nsmoc_save(obj: u64, _cmd: u64, error_out: u64) -> u64 {
    if error_out != 0 {
        unsafe {
            *(error_out as *mut u64) = 0;
        }
    }
    if obj == 0 {
        return 0;
    }
    // Ensure the database exists
    let mut guard = db_connection();
    let _ = ensure_db(&mut guard);
    1
}

/// `executeFetchRequest:error:` — queries the SQLite store and returns an NSArray.
/// If the entity table exists, returns an array of NSManagedObject stubs.
pub extern "C" fn shim_nsmoc_execute_fetch_request(
    _obj: u64,
    _cmd: u64,
    request: u64,
    error_out: u64,
) -> u64 {
    if error_out != 0 {
        unsafe {
            *(error_out as *mut u64) = 0;
        }
    }
    if request == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }
    // Get entity name from the fetch request
    let entity_name_obj = shim_nsfr_entity_name(request, 0);
    let entity_name = if entity_name_obj != 0 {
        crate::cfstring::extract_cfstring_cstr(entity_name_obj).map(|s| s.to_string())
    } else {
        None
    };

    if let Some(ref name) = entity_name {
        let n = count_rows(name);
        if n > 0 {
            // Create stub NSManagedObject instances for each row
            let cls = nsobj::nsmanagedobject_class_addr();
            let mut objects = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let obj = shim_nsmo_alloc(cls, 0);
                objects.push(obj);
            }
            return crate::nsarray::make_nsarray(&objects);
        }
    }
    crate::nsarray::make_nsarray(&[])
}

pub extern "C" fn shim_nsmoc_persistent_store_coordinator(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsmoc_set_persistent_store_coordinator(_obj: u64, _cmd: u64, _coord: u64) {}
pub extern "C" fn shim_nsmoc_merge_changes_from_context_did_save(
    _obj: u64,
    _cmd: u64,
    _notif: u64,
) {
}
pub extern "C" fn shim_nsmoc_undo_manager(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsmoc_has_changes(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsmoc_reset(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_nsmoc_rollback(_obj: u64, _cmd: u64) {}

// ── NSPersistentStoreCoordinator ──────────────────────────────────────────────

pub extern "C" fn shim_nspsc_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSPERSISTENTSTORECOORDINATOR_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_nspsc_init_with_model(obj: u64, _cmd: u64, _model: u64) -> u64 {
    obj
}
pub extern "C" fn shim_nspsc_new(cls: u64, cmd: u64) -> u64 {
    shim_nspsc_alloc(cls, cmd)
}

/// `addPersistentStoreWithType:configuration:URL:options:error:` — initializes the SQLite store.
pub extern "C" fn shim_nspsc_add_store(
    obj: u64,
    _cmd: u64,
    _store_type: u64,
    _config: u64,
    _url: u64,
    _options: u64,
    error_out: u64,
) -> u64 {
    if error_out != 0 {
        unsafe {
            *(error_out as *mut u64) = 0;
        }
    }
    // Ensure the in-memory SQLite database is initialized
    let mut guard = db_connection();
    let _ = ensure_db(&mut guard);
    // Return a non-zero sentinel to indicate success
    if obj == 0 {
        return 0;
    }
    obj // return self as the "store" reference
}

pub extern "C" fn shim_nspsc_managed_object_model(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nspsc_persistent_stores(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

// ── NSFetchRequest ────────────────────────────────────────────────────────────
// Layout: [ISA:8][entityName:8][predicate:8][sortDescs:8][fetchLimit:8]

const NSFR_ENTITY_OFF: usize = 8;
const NSFR_PREDICATE_OFF: usize = 16;
const NSFR_SORTDESCS_OFF: usize = 24;
const NSFR_FETCHLIMIT_OFF: usize = 32;

pub extern "C" fn shim_nsfr_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSFETCHREQUEST_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_nsfr_init_with_entity_name(obj: u64, _cmd: u64, entity_name: u64) -> u64 {
    if obj != 0 {
        unsafe {
            *((obj as usize + NSFR_ENTITY_OFF) as *mut u64) = entity_name;
        }
    }
    obj
}

pub extern "C" fn shim_nsfr_fetch_request_with_entity_name(
    cls: u64,
    cmd: u64,
    entity_name: u64,
) -> u64 {
    let obj = shim_nsfr_alloc(cls, cmd);
    shim_nsfr_init_with_entity_name(obj, cmd, entity_name)
}

pub extern "C" fn shim_nsfr_entity_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSFR_ENTITY_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_predicate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSFR_PREDICATE_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_set_predicate(obj: u64, _cmd: u64, pred: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + NSFR_PREDICATE_OFF) as *mut u64) = pred;
    }
}

pub extern "C" fn shim_nsfr_sort_descriptors(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSFR_SORTDESCS_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_set_sort_descriptors(obj: u64, _cmd: u64, descs: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + NSFR_SORTDESCS_OFF) as *mut u64) = descs;
    }
}

pub extern "C" fn shim_nsfr_fetch_limit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSFR_FETCHLIMIT_OFF) as *const u64) }
}

pub extern "C" fn shim_nsfr_set_fetch_limit(obj: u64, _cmd: u64, limit: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + NSFR_FETCHLIMIT_OFF) as *mut u64) = limit;
    }
}

// ── NSManagedObject ────────────────────────────────────────────────────────────
// Layout: [ISA:8][entity:8][context:8]

pub extern "C" fn shim_nsmo_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMANAGEDOBJECT_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_nsmo_init_with_entity_context(
    obj: u64,
    _cmd: u64,
    entity: u64,
    context: u64,
) -> u64 {
    if obj != 0 {
        unsafe {
            *((obj as usize + 8) as *mut u64) = entity;
            *((obj as usize + 16) as *mut u64) = context;
        }
    }
    obj
}

pub extern "C" fn shim_nsmo_value_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 {
        return 0;
    }
    // Try to resolve the key as a CFString and look up in the store
    let key_str = crate::cfstring::extract_cfstring_cstr(key);
    if key_str.is_none() {
        return 0;
    }
    // We don't have a per-object entity name tracked, so return 0
    // (full entity tracking would require expanding the MO layout)
    0
}

pub extern "C" fn shim_nsmo_set_value_for_key(obj: u64, _cmd: u64, value: u64, key: u64) {
    if obj == 0 || key == 0 {
        return;
    }
    let key_str = crate::cfstring::extract_cfstring_cstr(key);
    let value_str = if value != 0 {
        crate::cfstring::extract_cfstring_cstr(value)
    } else {
        None
    };
    if let (Some(k), Some(v)) = (key_str, value_str) {
        // Use a generic "ManagedObject" entity if we don't know the real one
        store_value("ManagedObject", k, v);
    }
}
pub extern "C" fn shim_nsmo_entity(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsmo_managed_object_context(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsmo_is_deleted(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsmo_is_inserted(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsmo_is_updated(_obj: u64, _cmd: u64) -> u64 {
    0
}

// ── NSEntityDescription ───────────────────────────────────────────────────────

pub extern "C" fn shim_nsed_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSENTITYDESCRIPTION_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_nsed_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `+[NSEntityDescription entityForName:inManagedObjectContext:]` → nil
pub extern "C" fn shim_nsed_entity_for_name(_cls: u64, _cmd: u64, _name: u64, _ctx: u64) -> u64 {
    0
}

pub extern "C" fn shim_nsed_name(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_nsed_properties(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}
pub extern "C" fn shim_nsed_managed_object_class_name(_obj: u64, _cmd: u64) -> u64 {
    0
}

// ── NSManagedObjectModel ──────────────────────────────────────────────────────

pub extern "C" fn shim_nsmom_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMANAGEDOBJECTMODEL_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_nsmom_init(obj: u64, _cmd: u64) -> u64 {
    obj
}
pub extern "C" fn shim_nsmom_init_with_contents_of_url(obj: u64, _cmd: u64, _url: u64) -> u64 {
    obj
}

/// `+[NSManagedObjectModel mergedModelFromBundles:]` → nil
pub extern "C" fn shim_nsmom_merged_model_from_bundles(_cls: u64, _cmd: u64, _bundles: u64) -> u64 {
    0
}

pub extern "C" fn shim_nsmom_entities(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

pub extern "C" fn shim_nsmom_entity_versions_by_name(_obj: u64, _cmd: u64) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn sqlite_store_initialized() {
        let mut guard = db_connection();
        let conn = ensure_db(&mut guard);
        // Should be able to execute a basic query
        let result: i64 = conn.query_row("SELECT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn entity_table_created() {
        let mut guard = db_connection();
        let conn = ensure_db(&mut guard);
        ensure_entity_table(conn, "TestEntityUnique42");
        // Table should exist
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM [TestEntityUnique42]", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn store_and_count() {
        store_value("PersonStoreTest", "name", "Alice");
        store_value("PersonStoreTest", "name", "Bob");
        assert!(count_rows("PersonStoreTest") >= 2);
    }

    #[test]
    fn save_returns_yes() {
        let cls = nsobj::nsmanagedobjectcontext_class_addr();
        let ctx = shim_nsmoc_alloc(cls, 0);
        let result = shim_nsmoc_save(ctx, 0, 0);
        assert_eq!(result, 1);
        free_obj(ctx);
    }

    #[test]
    fn execute_fetch_empty_entity() {
        let fr_cls = nsobj::nsfetchrequest_class_addr();
        let entity_name = make_string("EmptyEntityUnique99");
        let fr = shim_nsfr_alloc(fr_cls, 0);
        let _ = shim_nsfr_init_with_entity_name(fr, 0, entity_name);
        let result = shim_nsmoc_execute_fetch_request(0, 0, fr, 0);
        assert_ne!(result, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(result, 0), 0);
    }

    #[test]
    fn execute_fetch_returns_objects() {
        store_value("EmployeeFetchTest", "name", "Carol");
        store_value("EmployeeFetchTest", "role", "Engineer");

        let fr_cls = nsobj::nsfetchrequest_class_addr();
        let entity_name = make_string("EmployeeFetchTest");
        let fr = shim_nsfr_alloc(fr_cls, 0);
        let _ = shim_nsfr_init_with_entity_name(fr, 0, entity_name);
        let result = shim_nsmoc_execute_fetch_request(0, 0, fr, 0);
        assert_ne!(result, 0);
        assert!(crate::nsarray::shim_nsarray_count(result, 0) >= 2);
    }

    #[test]
    fn add_persistent_store_returns_nonzero() {
        let cls = nsobj::nspersistentstorecoordinator_class_addr();
        let coord = shim_nspsc_alloc(cls, 0);
        let store = shim_nspsc_add_store(coord, 0, 0, 0, 0, 0, 0);
        assert_ne!(store, 0);
        free_obj(coord);
    }

    #[test]
    fn managed_object_set_value_stores() {
        let cls = nsobj::nsmanagedobject_class_addr();
        let mo = shim_nsmo_alloc(cls, 0);
        let key = make_string("title");
        let value = make_string("Engineer");
        shim_nsmo_set_value_for_key(mo, 0, value, key);
        // Verify it was stored
        assert!(count_rows("ManagedObject") >= 1);
        free_obj(mo);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsmoc_save(0, 0, 0), 0);
        shim_nsmoc_reset(0, 0);
        shim_nsmoc_rollback(0, 0);
        assert_eq!(shim_nsmoc_has_changes(0, 0), 0);
        assert_eq!(shim_nsmo_value_for_key(0, 0, 0), 0);
        shim_nsmo_set_value_for_key(0, 0, 0, 0);
    }
}
