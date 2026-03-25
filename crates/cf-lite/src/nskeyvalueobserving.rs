//! Key-Value Observing (KVO) support for NSObject.
//!
//! Provides manual KVO: `addObserver:forKeyPath:options:context:`,
//! `removeObserver:forKeyPath:`, `willChangeValueForKey:`,
//! `didChangeValueForKey:`, and `observeValueForKeyPath:ofObject:change:context:`.
//!
//! Observers are stored in a global map keyed by `(object_addr, key_path)`.
//! When `didChangeValueForKey:` is called, all registered observers for that
//! key are notified via `observeValueForKeyPath:ofObject:change:context:`.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::cfstring;

// ── KVO option flags ───────────────────────────────────────────────

/// KVO option: include the new value in the change dictionary.
#[allow(dead_code)]
pub const NSK_VO_OPTION_NEW: u64 = 0x01;
/// KVO option: include the old value in the change dictionary.
#[allow(dead_code)]
pub const NSK_VO_OPTION_OLD: u64 = 0x02;
/// KVO option: fire an initial notification on registration.
pub const NSK_VO_OPTION_INITIAL: u64 = 0x04;

// ── KVO change kind ────────────────────────────────────────────────

/// Change kind: a value was set.
#[allow(dead_code)]
pub const NSK_VO_CHANGE_SETTING: u64 = 1;

// ── Observer entry ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct KvoEntry {
    observer: u64,
    #[allow(dead_code)]
    options: u64,
    context: u64,
}

/// Key for the observer map: (observed object address, key path string).
type KvoKey = (u64, String);

/// Global observer registry: maps (obj, keyPath) → list of observers.
static KVO_OBSERVERS: Mutex<Option<HashMap<KvoKey, Vec<KvoEntry>>>> = Mutex::new(None);

fn with_observers<R>(f: impl FnOnce(&mut HashMap<KvoKey, Vec<KvoEntry>>) -> R) -> R {
    let mut guard = KVO_OBSERVERS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

// ── Shim: addObserver:forKeyPath:options:context: ──────────────────

/// `- [NSObject addObserver:forKeyPath:options:context:]`
///
/// Type encoding: `v@:@@Q^v` (void, self, SEL, observer, keyPath, options, context)
///
/// Registers `observer` to receive KVO notifications when `self`'s property
/// at `keyPath` changes.
pub extern "C" fn shim_nsobject_add_observer(
    obj: u64,
    _sel: u64,
    observer: u64,
    key_path: u64,
    options: u64,
    context: u64,
) {
    if obj == 0 || observer == 0 || key_path == 0 {
        return;
    }

    let key_str = match cfstring::extract_cfstring_cstr(key_path) {
        Some(s) => s.to_string(),
        None => return,
    };

    let entry = KvoEntry {
        observer,
        options,
        context,
    };

    with_observers(|map| {
        map.entry((obj, key_str)).or_default().push(entry.clone());
    });

    // If NSKeyValueObservingOptionInitial is set, fire an immediate notification
    if options & NSK_VO_OPTION_INITIAL != 0 {
        dispatch_observation(observer, key_path, obj, 0, context);
    }
}

// ── Shim: removeObserver:forKeyPath: ───────────────────────────────

/// `- [NSObject removeObserver:forKeyPath:]`
///
/// Type encoding: `v@:@@` (void, self, SEL, observer, keyPath)
pub extern "C" fn shim_nsobject_remove_observer(
    obj: u64,
    _sel: u64,
    observer: u64,
    key_path: u64,
) {
    if obj == 0 || key_path == 0 {
        return;
    }

    let key_str = match cfstring::extract_cfstring_cstr(key_path) {
        Some(s) => s.to_string(),
        None => return,
    };

    with_observers(|map| {
        if let Some(entries) = map.get_mut(&(obj, key_str)) {
            entries.retain(|e| e.observer != observer);
        }
    });
}

// ── Shim: willChangeValueForKey: ───────────────────────────────────

/// `- [NSObject willChangeValueForKey:]`
///
/// Type encoding: `v@:@` (void, self, SEL, key)
///
/// Currently a no-op — we don't capture old values. The method exists
/// so fixture code can call it without crashing.
pub extern "C" fn shim_nsobject_will_change_value_for_key(
    _obj: u64,
    _sel: u64,
    _key: u64,
) {
    // No-op: could capture old values for NSKeyValueObservingOptionOld
}

// ── Shim: didChangeValueForKey: ────────────────────────────────────

/// `- [NSObject didChangeValueForKey:]`
///
/// Type encoding: `v@:@` (void, self, SEL, key)
///
/// Notifies all registered observers for `(self, key)` by calling
/// `observeValueForKeyPath:ofObject:change:context:` on each observer.
pub extern "C" fn shim_nsobject_did_change_value_for_key(
    obj: u64,
    _sel: u64,
    key: u64,
) {
    if obj == 0 || key == 0 {
        return;
    }

    let key_str = match cfstring::extract_cfstring_cstr(key) {
        Some(s) => s.to_string(),
        None => return,
    };

    // Collect matching observers under the lock, then dispatch outside the lock
    let entries: Vec<KvoEntry> = with_observers(|map| {
        map.get(&(obj, key_str)).cloned().unwrap_or_default()
    });

    for entry in &entries {
        // change dictionary is 0 (nil) for simplicity
        dispatch_observation(entry.observer, key, obj, 0, entry.context);
    }
}

// ── Shim: observeValueForKeyPath:ofObject:change:context: ──────────

/// `- [NSObject observeValueForKeyPath:ofObject:change:context:]`
///
/// Type encoding: `v@:@@@^v` (void, self, SEL, keyPath, object, change, context)
///
/// Default implementation is a no-op. Subclasses override this to handle
/// KVO notifications. When we dispatch to an observer, we use objc_msgSend
/// to call this method, which will find any override through normal dispatch.
pub extern "C" fn shim_nsobject_observe_value(
    _self_: u64,
    _sel: u64,
    _key_path: u64,
    _object: u64,
    _change: u64,
    _context: u64,
) {
    // Default NSObject implementation: no-op.
    // Subclass shims or guest code overrides handle this.
}

// ── Dispatch helper ────────────────────────────────────────────────

/// Call `[observer observeValueForKeyPath:key ofObject:obj change:nil context:ctx]`
/// through the ObjC runtime dispatch mechanism.
fn dispatch_observation(observer: u64, key_path: u64, object: u64, change: u64, context: u64) {
    use objc_runtime::OBJC_RUNTIME;

    if observer == 0 {
        return;
    }

    let isa = unsafe { *(observer as *const u64) };
    let sel_name = "observeValueForKeyPath:ofObject:change:context:";

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return;
    };

    let Some(imp) = rt.msg_lookup(isa, sel_name) else {
        return;
    };

    // Get the selector pointer for the dispatch
    let sel_ptr = rt.selector_ptr(sel_name).unwrap_or(0);

    // Drop the lock before calling the IMP to avoid deadlock
    drop(guard);

    // Call the IMP: void (*)(id self, SEL _cmd, id keyPath, id object, id change, void *context)
    let fn_ptr: extern "C" fn(u64, u64, u64, u64, u64, u64) =
        unsafe { std::mem::transmute(imp) };
    fn_ptr(observer, sel_ptr as u64, key_path, object, change, context);
}

// ── Query helpers (for tests / FFI) ────────────────────────────────

/// Returns the number of observers registered for a given (object, keyPath).
#[allow(dead_code)]
pub fn observer_count(obj: u64, key_path: &str) -> usize {
    with_observers(|map| {
        map.get(&(obj, key_path.to_string()))
            .map_or(0, |v| v.len())
    })
}

/// Clear all KVO registrations (for test isolation).
#[allow(dead_code)]
pub fn clear_all_observers() {
    with_observers(|map| map.clear());
}
