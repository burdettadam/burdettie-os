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
pub extern "C" fn shim_nsobject_remove_observer(obj: u64, _sel: u64, observer: u64, key_path: u64) {
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
pub extern "C" fn shim_nsobject_will_change_value_for_key(_obj: u64, _sel: u64, _key: u64) {
    // No-op: could capture old values for NSKeyValueObservingOptionOld
}

// ── Shim: didChangeValueForKey: ────────────────────────────────────

/// `- [NSObject didChangeValueForKey:]`
///
/// Type encoding: `v@:@` (void, self, SEL, key)
///
/// Notifies all registered observers for `(self, key)` by calling
/// `observeValueForKeyPath:ofObject:change:context:` on each observer.
pub extern "C" fn shim_nsobject_did_change_value_for_key(obj: u64, _sel: u64, key: u64) {
    if obj == 0 || key == 0 {
        return;
    }

    let key_str = match cfstring::extract_cfstring_cstr(key) {
        Some(s) => s.to_string(),
        None => return,
    };

    // Collect matching observers under the lock, then dispatch outside the lock
    let entries: Vec<KvoEntry> =
        with_observers(|map| map.get(&(obj, key_str)).cloned().unwrap_or_default());

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

    let guard = match OBJC_RUNTIME.read() {
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
    let fn_ptr: extern "C" fn(u64, u64, u64, u64, u64, u64) = unsafe { std::mem::transmute(imp) };
    fn_ptr(observer, sel_ptr as u64, key_path, object, change, context);
}

// ── Query helpers (for tests / FFI) ────────────────────────────────

/// Returns the number of observers registered for a given (object, keyPath).
#[allow(dead_code)]
pub fn observer_count(obj: u64, key_path: &str) -> usize {
    with_observers(|map| map.get(&(obj, key_path.to_string())).map_or(0, |v| v.len()))
}

/// Clear all KVO registrations (for test isolation).
#[allow(dead_code)]
pub fn clear_all_observers() {
    with_observers(|map| map.clear());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    /// Helper: reset global observer state before each logical test.
    fn reset() {
        clear_all_observers();
    }

    // ── addObserver:forKeyPath:options:context: ────────────────────

    #[test]
    fn add_observer_basic() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let key = make_string("name");

        shim_nsobject_add_observer(obj, 0, observer, key, 0, 0);
        assert_eq!(observer_count(obj, "name"), 1);

        free_obj(observer);
        free_obj(obj);
    }

    #[test]
    fn add_observer_multiple() {
        let obj = alloc_obj_fake(16);
        let obs1 = alloc_obj_fake(16);
        let obs2 = alloc_obj_fake(16);
        let key = make_string("score");

        shim_nsobject_add_observer(obj, 0, obs1, key, 0, 0);
        shim_nsobject_add_observer(obj, 0, obs2, key, 0, 0);
        assert_eq!(observer_count(obj, "score"), 2);

        free_obj(obs2);
        free_obj(obs1);
        free_obj(obj);
    }

    #[test]
    fn add_observer_different_keys() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let key_a = make_string("alpha");
        let key_b = make_string("beta");

        shim_nsobject_add_observer(obj, 0, observer, key_a, 0, 0);
        shim_nsobject_add_observer(obj, 0, observer, key_b, 0, 0);
        assert_eq!(observer_count(obj, "alpha"), 1);
        assert_eq!(observer_count(obj, "beta"), 1);

        free_obj(observer);
        free_obj(obj);
    }

    #[test]
    fn add_observer_with_initial_option() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let key = make_string("value");

        // NSK_VO_OPTION_INITIAL triggers dispatch_observation which will
        // gracefully no-op because the ObjC runtime is not initialized.
        shim_nsobject_add_observer(obj, 0, observer, key, NSK_VO_OPTION_INITIAL, 0);
        assert_eq!(observer_count(obj, "value"), 1);

        free_obj(observer);
        free_obj(obj);
    }

    #[test]
    fn add_observer_with_context() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let key = make_string("ctx");

        shim_nsobject_add_observer(obj, 0, observer, key, 0, 0xBEEF);
        assert_eq!(observer_count(obj, "ctx"), 1);

        free_obj(observer);
        free_obj(obj);
    }

    // ── addObserver nil safety ─────────────────────────────────────

    #[test]
    fn add_observer_nil_obj() {
        let observer = alloc_obj_fake(16);
        let key = make_string("x");
        shim_nsobject_add_observer(0, 0, observer, key, 0, 0);
        // Should be a no-op — nothing registered
        free_obj(observer);
    }

    #[test]
    fn add_observer_nil_observer() {
        let obj = alloc_obj_fake(16);
        let key = make_string("x");
        shim_nsobject_add_observer(obj, 0, 0, key, 0, 0);
        assert_eq!(observer_count(obj, "x"), 0);
        free_obj(obj);
    }

    #[test]
    fn add_observer_nil_key_path() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        shim_nsobject_add_observer(obj, 0, observer, 0, 0, 0);
        // No crash, nothing registered
        free_obj(observer);
        free_obj(obj);
    }

    #[test]
    fn add_observer_all_nil() {
        shim_nsobject_add_observer(0, 0, 0, 0, 0, 0);
        // Must not crash
    }

    // ── removeObserver:forKeyPath: ─────────────────────────────────

    #[test]
    fn remove_observer_basic() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let key = make_string("name");

        shim_nsobject_add_observer(obj, 0, observer, key, 0, 0);
        assert_eq!(observer_count(obj, "name"), 1);

        let key2 = make_string("name");
        shim_nsobject_remove_observer(obj, 0, observer, key2);
        assert_eq!(observer_count(obj, "name"), 0);

        free_obj(observer);
        free_obj(obj);
    }

    #[test]
    fn remove_observer_leaves_others() {
        let obj = alloc_obj_fake(16);
        let obs1 = alloc_obj_fake(16);
        let obs2 = alloc_obj_fake(16);
        let key = make_string("score");

        shim_nsobject_add_observer(obj, 0, obs1, key, 0, 0);
        let key2 = make_string("score");
        shim_nsobject_add_observer(obj, 0, obs2, key2, 0, 0);
        assert_eq!(observer_count(obj, "score"), 2);

        let key3 = make_string("score");
        shim_nsobject_remove_observer(obj, 0, obs1, key3);
        assert_eq!(observer_count(obj, "score"), 1);

        free_obj(obs2);
        free_obj(obs1);
        free_obj(obj);
    }

    #[test]
    fn remove_observer_double_remove() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let key = make_string("name");

        shim_nsobject_add_observer(obj, 0, observer, key, 0, 0);

        let k1 = make_string("name");
        shim_nsobject_remove_observer(obj, 0, observer, k1);
        assert_eq!(observer_count(obj, "name"), 0);

        // Second remove should be a no-op
        let k2 = make_string("name");
        shim_nsobject_remove_observer(obj, 0, observer, k2);
        assert_eq!(observer_count(obj, "name"), 0);

        free_obj(observer);
        free_obj(obj);
    }

    #[test]
    fn remove_observer_wrong_key() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let key = make_string("name");

        shim_nsobject_add_observer(obj, 0, observer, key, 0, 0);

        let wrong = make_string("other");
        shim_nsobject_remove_observer(obj, 0, observer, wrong);
        // Original registration untouched
        assert_eq!(observer_count(obj, "name"), 1);

        free_obj(observer);
        free_obj(obj);
    }

    // ── removeObserver nil safety ──────────────────────────────────

    #[test]
    fn remove_observer_nil_obj() {
        let observer = alloc_obj_fake(16);
        let key = make_string("x");
        shim_nsobject_remove_observer(0, 0, observer, key);
        free_obj(observer);
    }

    #[test]
    fn remove_observer_nil_key_path() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        shim_nsobject_remove_observer(obj, 0, observer, 0);
        free_obj(observer);
        free_obj(obj);
    }

    #[test]
    fn remove_observer_nil_observer() {
        let obj = alloc_obj_fake(16);
        let key = make_string("x");
        // nil observer — should not crash, effectively no-op
        shim_nsobject_remove_observer(obj, 0, 0, key);
        free_obj(obj);
    }

    #[test]
    fn remove_observer_all_nil() {
        shim_nsobject_remove_observer(0, 0, 0, 0);
    }

    // ── willChangeValueForKey: ─────────────────────────────────────

    #[test]
    fn will_change_value_noop() {
        let obj = alloc_obj_fake(16);
        let key = make_string("test");
        shim_nsobject_will_change_value_for_key(obj, 0, key);
        free_obj(obj);
    }

    #[test]
    fn will_change_value_nil_obj() {
        shim_nsobject_will_change_value_for_key(0, 0, 0);
    }

    // ── didChangeValueForKey: ──────────────────────────────────────

    #[test]
    fn did_change_value_no_observers() {
        let obj = alloc_obj_fake(16);
        let key = make_string("value");
        // No observers registered — should be a harmless no-op
        shim_nsobject_did_change_value_for_key(obj, 0, key);
        free_obj(obj);
    }

    #[test]
    fn did_change_value_nil_obj() {
        shim_nsobject_did_change_value_for_key(0, 0, 0);
    }

    #[test]
    fn did_change_value_nil_key() {
        let obj = alloc_obj_fake(16);
        shim_nsobject_did_change_value_for_key(obj, 0, 0);
        free_obj(obj);
    }

    // ── observeValueForKeyPath:ofObject:change:context: ────────────

    #[test]
    fn observe_value_default_noop() {
        shim_nsobject_observe_value(1, 0, 0, 0, 0, 0);
    }

    #[test]
    fn observe_value_all_nil() {
        shim_nsobject_observe_value(0, 0, 0, 0, 0, 0);
    }

    // ── observer_count & clear_all_observers helpers ───────────────

    #[test]
    fn observer_count_empty() {
        assert_eq!(observer_count(0xABC, "anything"), 0);
    }

    #[test]
    fn clear_all_observers_works() {
        let obj = alloc_obj_fake(16);
        let observer = alloc_obj_fake(16);
        let k1 = make_string("a");
        let k2 = make_string("b");

        shim_nsobject_add_observer(obj, 0, observer, k1, 0, 0);
        shim_nsobject_add_observer(obj, 0, observer, k2, 0, 0);
        assert_eq!(observer_count(obj, "a"), 1);
        assert_eq!(observer_count(obj, "b"), 1);

        clear_all_observers();
        assert_eq!(observer_count(obj, "a"), 0);
        assert_eq!(observer_count(obj, "b"), 0);

        free_obj(observer);
        free_obj(obj);
    }

    // ── Add-then-remove round trip ─────────────────────────────────

    #[test]
    fn add_remove_round_trip() {
        let obj = alloc_obj_fake(16);
        let obs = alloc_obj_fake(16);
        let key = make_string("count");

        shim_nsobject_add_observer(obj, 0, obs, key, 0, 42);
        assert_eq!(observer_count(obj, "count"), 1);

        let key2 = make_string("count");
        shim_nsobject_remove_observer(obj, 0, obs, key2);
        assert_eq!(observer_count(obj, "count"), 0);

        // Re-add should work
        let key3 = make_string("count");
        shim_nsobject_add_observer(obj, 0, obs, key3, 0, 99);
        assert_eq!(observer_count(obj, "count"), 1);

        free_obj(obs);
        free_obj(obj);
    }
}
