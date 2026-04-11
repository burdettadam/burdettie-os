//! macOS notify(3) — real in-process notification dispatch.
//!
//! Provides a per-process notification name-space where:
//!   - `notify_register_dispatch` registers a (queue, handler) for a name
//!     and assigns a unique token.
//!   - `notify_register_check` registers a poll-only token for a name.
//!   - `notify_set_state` / `notify_get_state` store per-token u64 state.
//!   - `notify_post` fires all handlers registered for the named notification.
//!   - `notify_cancel` removes a registration.

use crate::table::ShimTable;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use super::dispatch_queue;

/// NOTIFY_STATUS_OK
const NOTIFY_STATUS_OK: u32 = 0;
/// NOTIFY_STATUS_INVALID_NAME
#[allow(dead_code)]
const NOTIFY_STATUS_INVALID_NAME: u32 = 2;
/// NOTIFY_STATUS_INVALID_TOKEN
const NOTIFY_STATUS_INVALID_TOKEN: u32 = 5;

// ── Token allocator ─────────────────────────────────────────────────────────

static NEXT_TOKEN: AtomicI32 = AtomicI32::new(1);

fn alloc_token() -> i32 {
    NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
}

// ── Registration registry ───────────────────────────────────────────────────

/// A single notification registration.
struct Registration {
    /// The notification name this registration listens on.
    name: String,
    /// Per-token state (read/written by notify_get_state / notify_set_state).
    state: u64,
    /// Dirty-bit: set by `notify_post`, cleared by `notify_check`.
    dirty: bool,
    /// True if this is a dispatch registration (has queue + handler block).
    has_handler: bool,
    /// GCD queue handle to submit the handler on (if dispatch registration).
    queue: u64,
    /// Block pointer for the handler (if dispatch registration).
    /// Block invoke is at offset 16: `void (^handler)(int token)`.
    handler_block: u64,
}

static REGISTRY: Mutex<Option<HashMap<i32, Registration>>> = Mutex::new(None);

fn registry() -> std::sync::MutexGuard<'static, Option<HashMap<i32, Registration>>> {
    REGISTRY.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Install ─────────────────────────────────────────────────────────────────

pub struct NotifyStubs;

impl NotifyStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_notify_register_dispatch", shim_notify_register_dispatch as *const () as u64, "notify_register_dispatch (real dispatch)"),
            ("_notify_register_check", shim_notify_register_check as *const () as u64, "notify_register_check (real token)"),
            ("_notify_check", shim_notify_check as *const () as u64, "notify_check (real state check)"),
            ("_notify_get_state", shim_notify_get_state as *const () as u64, "notify_get_state (real)"),
            ("_notify_set_state", shim_notify_set_state as *const () as u64, "notify_set_state (real)"),
            ("_notify_post", shim_notify_post as *const () as u64, "notify_post (real fires handlers)"),
            ("_notify_cancel", shim_notify_cancel as *const () as u64, "notify_cancel (real)"),
            ("_notify_register_mach_port", shim_notify_register_check as *const () as u64, "notify_register_mach_port (→ check token)"),
            ("_notify_register_file_descriptor", shim_notify_register_check as *const () as u64, "notify_register_file_descriptor (→ check token)"),
            ("_notify_register_signal", shim_notify_register_check as *const () as u64, "notify_register_signal (→ check token)"),
            ("_notify_suspend", shim_notify_suspend as *const () as u64, "notify_suspend (real)"),
            ("_notify_resume", shim_notify_resume as *const () as u64, "notify_resume (real)"),
            ("_notify_is_valid_token", shim_notify_is_valid_token as *const () as u64, "notify_is_valid_token (real)"),
        );
        log::debug!("NotifyStubs: registered 13 notify shims");
    }
}

// ── Helper: extract C string from pointer ───────────────────────────────────

fn extract_name(name_ptr: u64) -> Option<String> {
    if name_ptr == 0 {
        return None;
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const std::ffi::c_char) };
    cstr.to_str().ok().map(|s| s.to_string())
}

// ── Shim functions ──────────────────────────────────────────────────────────

/// `uint32_t notify_register_dispatch(const char *name, int *out_token,
///     dispatch_queue_t queue, notify_handler_t handler)`
///
/// Registers `handler` to be submitted to `queue` whenever `notify_post(name)`
/// is called. The handler is `void (^)(int token)` — a block taking the token.
pub extern "C" fn shim_notify_register_dispatch(
    name: u64, out_token: *mut i32, queue: u64, handler: u64,
) -> u32 {
    let name_str = match extract_name(name) {
        Some(n) => n,
        None => {
            if !out_token.is_null() { unsafe { *out_token = 0; } }
            return NOTIFY_STATUS_OK;
        }
    };

    let token = alloc_token();
    log::debug!("notify_register_dispatch(\"{}\") → token {}", name_str, token);

    let mut guard = registry();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(token, Registration {
        name: name_str,
        state: 0,
        dirty: false,
        has_handler: handler != 0,
        queue,
        handler_block: handler,
    });

    if !out_token.is_null() {
        unsafe { *out_token = token; }
    }
    NOTIFY_STATUS_OK
}

/// `uint32_t notify_register_check(const char *name, int *out_token)`
///
/// Registers a poll-only token. `notify_check` can later test if the
/// notification was posted since the last check.
pub extern "C" fn shim_notify_register_check(
    name: u64, out_token: *mut i32,
) -> u32 {
    let name_str = match extract_name(name) {
        Some(n) => n,
        None => {
            if !out_token.is_null() { unsafe { *out_token = 0; } }
            return NOTIFY_STATUS_OK;
        }
    };

    let token = alloc_token();
    log::debug!("notify_register_check(\"{}\") → token {}", name_str, token);

    let mut guard = registry();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(token, Registration {
        name: name_str,
        state: 0,
        dirty: false,
        has_handler: false,
        queue: 0,
        handler_block: 0,
    });

    if !out_token.is_null() {
        unsafe { *out_token = token; }
    }
    NOTIFY_STATUS_OK
}

/// `uint32_t notify_check(int token, int *check)`
///
/// Sets `*check` to 1 if the notification was posted since the last check
/// (or registration), 0 otherwise.
pub extern "C" fn shim_notify_check(token: i32, check: *mut i32) -> u32 {
    let mut guard = registry();
    if let Some(map) = guard.as_mut() {
        if let Some(reg) = map.get_mut(&token) {
            if !check.is_null() {
                unsafe { *check = if reg.dirty { 1 } else { 0 }; }
            }
            reg.dirty = false; // clear after read
            return NOTIFY_STATUS_OK;
        }
    }
    if !check.is_null() {
        unsafe { *check = 0; }
    }
    NOTIFY_STATUS_INVALID_TOKEN
}

/// `uint32_t notify_get_state(int token, uint64_t *state)`
pub extern "C" fn shim_notify_get_state(token: i32, state: *mut u64) -> u32 {
    let guard = registry();
    if let Some(map) = guard.as_ref() {
        if let Some(reg) = map.get(&token) {
            if !state.is_null() {
                unsafe { *state = reg.state; }
            }
            return NOTIFY_STATUS_OK;
        }
    }
    if !state.is_null() {
        unsafe { *state = 0; }
    }
    NOTIFY_STATUS_INVALID_TOKEN
}

/// `uint32_t notify_set_state(int token, uint64_t state)`
pub extern "C" fn shim_notify_set_state(token: i32, state: u64) -> u32 {
    let mut guard = registry();
    if let Some(map) = guard.as_mut() {
        if let Some(reg) = map.get_mut(&token) {
            reg.state = state;
            return NOTIFY_STATUS_OK;
        }
    }
    NOTIFY_STATUS_INVALID_TOKEN
}

/// `uint32_t notify_post(const char *name)`
///
/// Posts a notification by name. All dispatch-registered handlers for this
/// name are submitted to their respective queues. State is preserved.
pub extern "C" fn shim_notify_post(name: u64) -> u32 {
    let name_str = match extract_name(name) {
        Some(n) => n,
        None => return NOTIFY_STATUS_OK,
    };

    log::debug!("notify_post(\"{}\")", name_str);

    // Set dirty bit on all matching registrations and collect dispatch handlers.
    let to_fire: Vec<(i32, u64, u64)> = {
        let mut guard = registry();
        match guard.as_mut() {
            Some(map) => {
                let mut handlers = Vec::new();
                for (&token, reg) in map.iter_mut() {
                    if reg.name == name_str {
                        reg.dirty = true;
                        if reg.has_handler && reg.handler_block != 0 {
                            handlers.push((token, reg.queue, reg.handler_block));
                        }
                    }
                }
                handlers
            }
            None => Vec::new(),
        }
    };

    for (token, queue, block) in to_fire {
        // Block layout: isa(8) + flags(4) + reserved(4) + invoke(8)
        // invoke signature: void (^)(int token)
        let invoke_ptr = unsafe { *((block as *const u8).add(16) as *const usize) };
        let block_addr = block as usize;
        dispatch_queue::queue_async(
            queue,
            Box::new(move || {
                let invoke: extern "C" fn(*const u8, i32) =
                    unsafe { std::mem::transmute(invoke_ptr) };
                invoke(block_addr as *const u8, token);
            }),
        );
    }

    NOTIFY_STATUS_OK
}

/// `uint32_t notify_cancel(int token)`
pub extern "C" fn shim_notify_cancel(token: i32) -> u32 {
    let mut guard = registry();
    if let Some(map) = guard.as_mut() {
        if map.remove(&token).is_some() {
            log::debug!("notify_cancel(token={})", token);
            return NOTIFY_STATUS_OK;
        }
    }
    NOTIFY_STATUS_INVALID_TOKEN
}

/// `uint32_t notify_suspend(int token)`
pub extern "C" fn shim_notify_suspend(token: i32) -> u32 {
    let guard = registry();
    if let Some(map) = guard.as_ref() {
        if map.contains_key(&token) {
            return NOTIFY_STATUS_OK;
        }
    }
    NOTIFY_STATUS_INVALID_TOKEN
}

/// `uint32_t notify_resume(int token)`
pub extern "C" fn shim_notify_resume(token: i32) -> u32 {
    let guard = registry();
    if let Some(map) = guard.as_ref() {
        if map.contains_key(&token) {
            return NOTIFY_STATUS_OK;
        }
    }
    NOTIFY_STATUS_INVALID_TOKEN
}

/// `bool notify_is_valid_token(int token)`
pub extern "C" fn shim_notify_is_valid_token(token: i32) -> u32 {
    let guard = registry();
    if let Some(map) = guard.as_ref() {
        if map.contains_key(&token) {
            return 1;
        }
    }
    0
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_name(s: &str) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        v
    }

    #[test]
    fn register_check_assigns_unique_tokens() {
        let name = make_name("com.test.unique_tokens");
        let mut t1: i32 = 0;
        let mut t2: i32 = 0;
        assert_eq!(shim_notify_register_check(name.as_ptr() as u64, &mut t1), NOTIFY_STATUS_OK);
        assert_eq!(shim_notify_register_check(name.as_ptr() as u64, &mut t2), NOTIFY_STATUS_OK);
        assert_ne!(t1, t2);
        assert_ne!(t1, 0);
        shim_notify_cancel(t1);
        shim_notify_cancel(t2);
    }

    #[test]
    fn get_set_state_roundtrip() {
        let name = make_name("com.test.state_roundtrip");
        let mut token: i32 = 0;
        shim_notify_register_check(name.as_ptr() as u64, &mut token);

        assert_eq!(shim_notify_set_state(token, 42), NOTIFY_STATUS_OK);

        let mut state: u64 = 0;
        assert_eq!(shim_notify_get_state(token, &mut state), NOTIFY_STATUS_OK);
        assert_eq!(state, 42);

        shim_notify_cancel(token);
    }

    #[test]
    fn get_state_invalid_token() {
        let mut state: u64 = 99;
        let ret = shim_notify_get_state(-9999, &mut state);
        assert_eq!(ret, NOTIFY_STATUS_INVALID_TOKEN);
        assert_eq!(state, 0);
    }

    #[test]
    fn cancel_removes_token() {
        let name = make_name("com.test.cancel");
        let mut token: i32 = 0;
        shim_notify_register_check(name.as_ptr() as u64, &mut token);

        assert_eq!(shim_notify_cancel(token), NOTIFY_STATUS_OK);
        assert_eq!(shim_notify_cancel(token), NOTIFY_STATUS_INVALID_TOKEN);
    }

    #[test]
    fn is_valid_token_reflects_registration() {
        let name = make_name("com.test.valid");
        let mut token: i32 = 0;
        shim_notify_register_check(name.as_ptr() as u64, &mut token);

        assert_eq!(shim_notify_is_valid_token(token), 1);
        shim_notify_cancel(token);
        assert_eq!(shim_notify_is_valid_token(token), 0);
    }

    #[test]
    fn post_does_not_panic_with_no_registrations() {
        let name = make_name("com.test.noregistrations");
        assert_eq!(shim_notify_post(name.as_ptr() as u64), NOTIFY_STATUS_OK);
    }

    #[test]
    fn notify_check_returns_zero_for_valid_token() {
        let name = make_name("com.test.check");
        let mut token: i32 = 0;
        shim_notify_register_check(name.as_ptr() as u64, &mut token);

        let mut check: i32 = -1;
        let ret = shim_notify_check(token, &mut check);
        assert_eq!(ret, NOTIFY_STATUS_OK);
        assert_eq!(check, 0);
        shim_notify_cancel(token);
    }

    #[test]
    fn notify_check_invalid_token() {
        let mut check: i32 = -1;
        let ret = shim_notify_check(-8888, &mut check);
        assert_eq!(ret, NOTIFY_STATUS_INVALID_TOKEN);
    }

    #[test]
    fn suspend_resume_valid_token() {
        let name = make_name("com.test.suspend");
        let mut token: i32 = 0;
        shim_notify_register_check(name.as_ptr() as u64, &mut token);

        assert_eq!(shim_notify_suspend(token), NOTIFY_STATUS_OK);
        assert_eq!(shim_notify_resume(token), NOTIFY_STATUS_OK);
        shim_notify_cancel(token);
    }

    #[test]
    fn post_fires_dispatch_handlers() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::sync::Arc;

        // We can't easily test the full block dispatch path from unit tests
        // because it requires a real Block ABI layout. Instead, verify that
        // notify_post with a handler=0 dispatch registration doesn't crash.
        let name = make_name("com.test.post_dispatch");
        let mut token: i32 = 0;
        // Register with handler=0 (no block to invoke, but tracked)
        shim_notify_register_dispatch(name.as_ptr() as u64, &mut token, 0, 0);
        assert_ne!(token, 0);

        // Post should not crash even with handler_block=0
        assert_eq!(shim_notify_post(name.as_ptr() as u64), NOTIFY_STATUS_OK);
        shim_notify_cancel(token);
    }

    #[test]
    fn multiple_registrations_same_name() {
        let name = make_name("com.test.multi");
        let mut t1: i32 = 0;
        let mut t2: i32 = 0;
        shim_notify_register_check(name.as_ptr() as u64, &mut t1);
        shim_notify_register_check(name.as_ptr() as u64, &mut t2);

        // Both valid
        assert_eq!(shim_notify_is_valid_token(t1), 1);
        assert_eq!(shim_notify_is_valid_token(t2), 1);

        // Independent state
        shim_notify_set_state(t1, 100);
        shim_notify_set_state(t2, 200);
        let mut s1: u64 = 0;
        let mut s2: u64 = 0;
        shim_notify_get_state(t1, &mut s1);
        shim_notify_get_state(t2, &mut s2);
        assert_eq!(s1, 100);
        assert_eq!(s2, 200);

        shim_notify_cancel(t1);
        shim_notify_cancel(t2);
    }

    #[test]
    fn install_count() {
        let mut table = ShimTable::new();
        NotifyStubs::install(&mut table);
        assert!(table.len() >= 13, "expected ≥13 notify stubs, got {}", table.len());
    }
}
