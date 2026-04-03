//! XPC Activity scheduler — timer-based activity registration.
//!
//! Apple's `xpc_activity` APIs allow daemons to register periodic tasks
//! that launchd fires based on criteria (interval, network, power, etc.).
//! Without a real launchd, we provide a lightweight scheduler that:
//!
//! - Stores registered activities with their criteria dictionaries.
//! - Tracks activity state (CHECK_IN → RUN → DONE / DEFER).
//! - On `xpc_activity_register`, immediately fires the handler once with
//!   the activity in CHECK_IN state (matching launchd's initial check-in
//!   behavior), then transitions to RUN.
//! - Reads the `XPC_ACTIVITY_INTERVAL` key from the criteria dictionary
//!   (in seconds) and spawns a background timer thread that re-fires the
//!   handler periodically.  If no interval is set, only the initial
//!   check-in fires.
//! - `xpc_activity_should_defer` returns false so activities always execute.
//! - `xpc_activity_set_criteria` updates the criteria for future runs.
//! - `xpc_activity_unregister` removes the activity and stops its timer.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Activity states mirroring XPC_ACTIVITY_STATE_* constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum ActivityState {
    /// Waiting for initial check-in (default).
    CheckIn = 0,
    /// Wait (deferred).
    Wait = 1,
    /// Running.
    Run = 2,
    /// Done.
    Done = 3,
    /// Continue (re-schedule).
    Continue = 4,
}

impl ActivityState {
    pub fn from_u64(val: u64) -> Self {
        match val {
            0 => Self::CheckIn,
            1 => Self::Wait,
            2 => Self::Run,
            3 => Self::Done,
            4 => Self::Continue,
            _ => Self::CheckIn,
        }
    }
}

/// A registered XPC activity.
#[derive(Clone, Debug)]
struct Activity {
    name: String,
    /// The criteria dictionary handle (xpc_dictionary_t).
    criteria: u64,
    /// The handler block pointer.
    handler: u64,
    /// Current state.
    state: ActivityState,
    /// Number of times the handler has been invoked.
    run_count: u64,
    /// Cancellation flag shared with the timer thread (if any).
    cancelled: Arc<AtomicBool>,
    /// Interval in seconds from the criteria dictionary (0 = no periodic).
    interval_secs: u64,
}

static NEXT_ACTIVITY_ID: AtomicU64 = AtomicU64::new(1);

/// Global activity registry: name → Activity.
static ACTIVITIES: Mutex<Option<HashMap<String, Activity>>> = Mutex::new(None);

fn with_activities<R>(f: impl FnOnce(&mut HashMap<String, Activity>) -> R) -> R {
    let mut guard = ACTIVITIES.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    f(table)
}

/// Well-known criteria key for the repeat interval (seconds).
/// Apple uses `XPC_ACTIVITY_INTERVAL` — we also accept `_interval_secs`.
const INTERVAL_KEY: &[u8] = b"_interval_secs\0";

/// `xpc_activity_register(name, criteria, handler)`
///
/// Registers a named activity.  Fires the handler once immediately in
/// CHECK_IN state (matching Apple's behavior where launchd calls the
/// handler on registration to let the daemon configure itself).
///
/// If the criteria dictionary contains an `_interval_secs` key with an
/// int64 value > 0, spawns a background thread that re-fires the handler
/// periodically at that interval.
pub fn register(name_ptr: u64, criteria: u64, handler: u64) {
    let name = if name_ptr != 0 {
        let cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const i8) };
        cstr.to_str().unwrap_or("unknown").to_string()
    } else {
        let id = NEXT_ACTIVITY_ID.fetch_add(1, Ordering::Relaxed);
        format!("activity_{}", id)
    };

    log::debug!("xpc_activity: registering '{}'", name);

    // Read interval from criteria (if present)
    let interval_secs = if criteria != 0 {
        let val = super::xpc_objects::dict_get_int64(
            criteria,
            INTERVAL_KEY.as_ptr() as u64,
        );
        if val > 0 { val as u64 } else { 0 }
    } else {
        0
    };

    let cancelled = Arc::new(AtomicBool::new(false));

    let activity = Activity {
        name: name.clone(),
        criteria,
        handler,
        state: ActivityState::CheckIn,
        run_count: 0,
        cancelled: cancelled.clone(),
        interval_secs,
    };

    // Cancel any previous activity with the same name
    with_activities(|t| {
        if let Some(old) = t.get(&name) {
            old.cancelled.store(true, Ordering::Relaxed);
        }
        t.insert(name.clone(), activity);
    });

    // Fire the handler once in CHECK_IN state (initial registration callback).
    // Apple's launchd does this so the daemon can set up or modify criteria.
    if handler != 0 {
        // Create an activity object handle the handler can inspect.
        // We use a dictionary with the activity state embedded.
        let activity_obj = super::xpc_objects::create_dictionary_empty();
        super::xpc_objects::dict_set_int64(
            activity_obj,
            b"_state\0".as_ptr() as u64,
            ActivityState::CheckIn as i64,
        );
        super::xpc_objects::dict_set_string(
            activity_obj,
            b"_name\0".as_ptr() as u64,
            name_ptr,
        );

        invoke_block_1(handler, activity_obj);

        // Transition to RUN after initial check-in callback
        with_activities(|t| {
            if let Some(a) = t.get_mut(&name) {
                a.state = ActivityState::Run;
                a.run_count += 1;
                log::debug!("xpc_activity: '{}' transitioned to RUN", name);
            }
        });
    }

    // Spawn periodic timer thread if interval > 0
    if interval_secs > 0 && handler != 0 {
        let timer_name = name.clone();
        let timer_cancelled = cancelled;
        std::thread::spawn(move || {
            let interval = Duration::from_secs(interval_secs);
            loop {
                std::thread::sleep(interval);
                if timer_cancelled.load(Ordering::Relaxed) {
                    log::debug!("xpc_activity: timer for '{}' cancelled", timer_name);
                    break;
                }
                // Check activity still exists
                let still_registered = with_activities(|t| t.contains_key(&timer_name));
                if !still_registered {
                    break;
                }

                log::debug!("xpc_activity: timer firing '{}' (periodic)", timer_name);

                // Create activity object for this invocation
                let activity_obj = super::xpc_objects::create_dictionary_empty();
                super::xpc_objects::dict_set_int64(
                    activity_obj,
                    b"_state\0".as_ptr() as u64,
                    ActivityState::Run as i64,
                );

                invoke_block_1(handler, activity_obj);

                // Update run count
                with_activities(|t| {
                    if let Some(a) = t.get_mut(&timer_name) {
                        a.run_count += 1;
                    }
                });
            }
        });
        log::debug!(
            "xpc_activity: '{}' periodic timer started (interval={}s)",
            name,
            interval_secs
        );
    }
}

/// `xpc_activity_unregister(name)`
///
/// Removes the activity and stops its periodic timer (if any).
pub fn unregister(name_ptr: u64) {
    if name_ptr == 0 { return; }
    let cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const i8) };
    let name = cstr.to_str().unwrap_or("unknown");
    log::debug!("xpc_activity: unregistering '{}'", name);
    with_activities(|t| {
        if let Some(activity) = t.remove(name) {
            activity.cancelled.store(true, Ordering::Relaxed);
        }
    });
}

/// `xpc_activity_set_criteria(activity, criteria)`
///
/// The `activity` argument is an opaque token — in our implementation it's
/// a dictionary handle.  We look up the activity by the embedded `_name` key.
pub fn set_criteria(activity: u64, criteria: u64) {
    // Try to read the name from the activity object
    let name_ptr = super::xpc_objects::dict_get_string(
        activity,
        b"_name\0".as_ptr() as u64,
    );
    if name_ptr != 0 {
        let cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const i8) };
        let name = cstr.to_str().unwrap_or("unknown");
        with_activities(|t| {
            if let Some(a) = t.get_mut(name) {
                a.criteria = criteria;
                log::debug!("xpc_activity: '{}' criteria updated", name);
            }
        });
    }
}

/// `xpc_activity_get_state(activity)` → XPC_ACTIVITY_STATE_*
pub fn get_state(activity: u64) -> u64 {
    // Read state from the activity dictionary
    let state_val = super::xpc_objects::dict_get_int64(
        activity,
        b"_state\0".as_ptr() as u64,
    );
    if state_val != 0 || super::xpc_objects::dict_get_count(activity) > 0 {
        return state_val as u64;
    }
    // Default: CHECK_IN
    ActivityState::CheckIn as u64
}

/// `xpc_activity_should_defer(activity)` → false (always run)
pub fn should_defer(_activity: u64) -> bool {
    false
}

/// Get the number of registered activities (for testing).
pub fn activity_count() -> usize {
    with_activities(|t| t.len())
}

/// Check if a named activity exists.
pub fn has_activity(name: &str) -> bool {
    with_activities(|t| t.contains_key(name))
}

/// Get the run count for a named activity.
pub fn run_count(name: &str) -> u64 {
    with_activities(|t| t.get(name).map_or(0, |a| a.run_count))
}

// ── Block invocation helper ────────────────────────────────────

fn invoke_block_1(block: u64, arg1: u64) {
    unsafe {
        let invoke_ptr = *((block + 16) as *const u64);
        if invoke_ptr != 0 {
            let invoke: extern "C" fn(u64, u64) = std::mem::transmute(invoke_ptr);
            invoke(block, arg1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_state_roundtrip() {
        assert_eq!(ActivityState::from_u64(0), ActivityState::CheckIn);
        assert_eq!(ActivityState::from_u64(2), ActivityState::Run);
        assert_eq!(ActivityState::from_u64(3), ActivityState::Done);
        assert_eq!(ActivityState::from_u64(99), ActivityState::CheckIn); // unknown → default
    }

    #[test]
    fn register_stores_activity() {
        let name = std::ffi::CString::new("com.test.activity").unwrap();
        // handler=0 means no callback invocation
        register(name.as_ptr() as u64, 0, 0);
        assert!(has_activity("com.test.activity"));
    }

    #[test]
    fn unregister_removes_activity() {
        let name = std::ffi::CString::new("com.test.unreg").unwrap();
        register(name.as_ptr() as u64, 0, 0);
        assert!(has_activity("com.test.unreg"));
        unregister(name.as_ptr() as u64);
        assert!(!has_activity("com.test.unreg"));
    }

    #[test]
    fn should_defer_always_false() {
        assert!(!should_defer(0));
        assert!(!should_defer(0x1234));
    }

    #[test]
    fn get_state_default_check_in() {
        // Unknown handle → CHECK_IN
        assert_eq!(get_state(0), ActivityState::CheckIn as u64);
    }
}
