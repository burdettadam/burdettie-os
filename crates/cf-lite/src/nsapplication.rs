//! NSApplication shim: application lifecycle singleton.
//!
//! Object layout (singleton):
//!
//!   offset  0: isa             (u64 — pointer to NSApplication class)
//!   offset  8: delegate_ptr    (u64 — delegate object, or 0)
//!   offset 16: main_menu_ptr   (u64 — NSMenu object, or 0)
//!   offset 24: is_running      (u64 — 1 if run loop active, 0 otherwise)
//!   offset 32: key_window_ptr  (u64 — current key NSWindow, or 0)
//!   offset 40: main_window_ptr (u64 — current main NSWindow, or 0)
//!   offset 48: current_event_ptr (u64 — current NSEvent, or 0)
//!
//! Total: 56 bytes.

use std::sync::{Mutex, OnceLock};

const DELEGATE_OFF: usize = 8;
const MAIN_MENU_OFF: usize = 16;
const IS_RUNNING_OFF: usize = 24;
const KEY_WINDOW_OFF: usize = 32;
const MAIN_WINDOW_OFF: usize = 40;
const CURRENT_EVENT_OFF: usize = 48;
const APP_SIZE: usize = 56;

static SHARED_APP: OnceLock<u64> = OnceLock::new();
static APP_WINDOWS: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();

fn app_windows() -> &'static Mutex<Vec<u64>> {
    APP_WINDOWS.get_or_init(|| Mutex::new(Vec::new()))
}

fn snapshot_windows() -> Vec<u64> {
    app_windows()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn snapshot_ordered_windows() -> Vec<u64> {
    snapshot_windows()
        .into_iter()
        .rev()
        .filter(|&window| crate::nswindow::is_window_visible(window))
        .collect()
}

fn clear_registered_windows() {
    app_windows().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

fn read_window_slot(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_window_slot(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

pub(crate) fn register_window(window: u64) {
    if window == 0 {
        return;
    }
    let mut windows = app_windows().lock().unwrap_or_else(|e| e.into_inner());
    if !windows.contains(&window) {
        windows.push(window);
    }
}

pub(crate) fn promote_window(window: u64) {
    if window == 0 {
        return;
    }

    let mut windows = app_windows().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(index) = windows.iter().position(|&candidate| candidate == window) {
        windows.remove(index);
    }
    windows.push(window);
}

pub(crate) fn current_key_window() -> u64 {
    let app = get_or_create_shared_app();
    read_window_slot(app, KEY_WINDOW_OFF)
}

pub(crate) fn current_main_window() -> u64 {
    let app = get_or_create_shared_app();
    read_window_slot(app, MAIN_WINDOW_OFF)
}

pub(crate) fn current_event() -> u64 {
    let app = get_or_create_shared_app();
    read_window_slot(app, CURRENT_EVENT_OFF)
}

pub(crate) fn set_key_window(window: u64) {
    if window != 0 {
        register_window(window);
    }
    let app = get_or_create_shared_app();
    write_window_slot(app, KEY_WINDOW_OFF, window);
}

pub(crate) fn set_main_window(window: u64) {
    if window != 0 {
        register_window(window);
    }
    let app = get_or_create_shared_app();
    write_window_slot(app, MAIN_WINDOW_OFF, window);
}

pub(crate) fn set_current_event(event: u64) {
    let app = get_or_create_shared_app();
    write_window_slot(app, CURRENT_EVENT_OFF, event);
}

pub(crate) fn push_current_event(event: u64) -> u64 {
    let previous = current_event();
    if event != 0 {
        set_current_event(event);
    }
    previous
}

pub(crate) fn restore_previous_current_event(previous: u64, dispatched_event: u64) {
    if previous != 0 && dispatched_event != 0 {
        set_current_event(previous);
    }
}

pub(crate) fn clear_window_roles(window: u64) {
    if window == 0 {
        return;
    }
    let app = get_or_create_shared_app();
    if read_window_slot(app, KEY_WINDOW_OFF) == window {
        write_window_slot(app, KEY_WINDOW_OFF, 0);
    }
    if read_window_slot(app, MAIN_WINDOW_OFF) == window {
        write_window_slot(app, MAIN_WINDOW_OFF, 0);
    }
}

pub(crate) fn reset_runtime_state_for_reuse() {
    clear_registered_windows();
    if let Some(app) = SHARED_APP.get().copied() {
        unsafe {
            *((app as usize + DELEGATE_OFF) as *mut u64) = 0;
            *((app as usize + MAIN_MENU_OFF) as *mut u64) = 0;
            *((app as usize + IS_RUNNING_OFF) as *mut u64) = 0;
            *((app as usize + KEY_WINDOW_OFF) as *mut u64) = 0;
            *((app as usize + MAIN_WINDOW_OFF) as *mut u64) = 0;
            *((app as usize + CURRENT_EVENT_OFF) as *mut u64) = 0;
        }
    }
}

fn get_or_create_shared_app() -> u64 {
    *SHARED_APP.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, APP_SIZE) as u64;
        if obj == 0 {
            return 0;
        }
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsapplication_class_addr();
        }
        log::debug!("NSApplication sharedApplication: created singleton at {:#x}", obj);
        obj
    })
}

/// Address of the NSApp global variable (pointer to the singleton).
static NSAPP_GLOBAL: OnceLock<u64> = OnceLock::new();

/// Returns the address of the `NSApp` global variable.
/// This is a pointer-to-pointer: the global holds the address of the shared app.
pub fn nsapp_global_addr() -> u64 {
    *NSAPP_GLOBAL.get_or_init(|| {
        let slot = crate::pal_alloc().calloc(1, 8) as *mut u64;
        let app = get_or_create_shared_app();
        unsafe {
            *slot = app;
        }
        slot as u64
    })
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSApplication sharedApplication]`
pub extern "C" fn shim_nsapplication_shared_application(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_shared_app()
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSApplication delegate]`
pub extern "C" fn shim_nsapplication_delegate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) }
}

/// `-[NSApplication setDelegate:]`
pub extern "C" fn shim_nsapplication_set_delegate(obj: u64, _cmd: u64, delegate: u64) {
    if obj == 0 {
        return;
    }
    unsafe { *((obj as usize + DELEGATE_OFF) as *mut u64) = delegate; }
    log::debug!("NSApplication setDelegate: {:#x}", delegate);
}

/// `-[NSApplication mainMenu]`
pub extern "C" fn shim_nsapplication_main_menu(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + MAIN_MENU_OFF) as *const u64) }
}

/// `-[NSApplication setMainMenu:]`
pub extern "C" fn shim_nsapplication_set_main_menu(obj: u64, _cmd: u64, menu: u64) {
    if obj == 0 {
        return;
    }
    unsafe { *((obj as usize + MAIN_MENU_OFF) as *mut u64) = menu; }
}

/// `-[NSApplication keyWindow]`
pub extern "C" fn shim_nsapplication_key_window(obj: u64, _cmd: u64) -> u64 {
    read_window_slot(obj, KEY_WINDOW_OFF)
}

/// `-[NSApplication mainWindow]`
pub extern "C" fn shim_nsapplication_main_window(obj: u64, _cmd: u64) -> u64 {
    read_window_slot(obj, MAIN_WINDOW_OFF)
}

/// `-[NSApplication currentEvent]`
pub extern "C" fn shim_nsapplication_current_event(obj: u64, _cmd: u64) -> u64 {
    read_window_slot(obj, CURRENT_EVENT_OFF)
}

/// `-[NSApplication targetForAction:]`
pub extern "C" fn shim_nsapplication_target_for_action(obj: u64, _cmd: u64, action: u64) -> u64 {
    if obj == 0 || action == 0 {
        return 0;
    }

    resolved_action_target_for_selector(action, 0)
}

/// `-[NSApplication targetForAction:to:from:]`
pub extern "C" fn shim_nsapplication_target_for_action_to_from(
    obj: u64,
    _cmd: u64,
    action: u64,
    target: u64,
    _sender: u64,
) -> u64 {
    if obj == 0 || action == 0 {
        return 0;
    }

    resolved_action_target_for_selector(action, target)
}

/// `-[NSApplication sendEvent:]`
pub extern "C" fn shim_nsapplication_send_event(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let previous_event = push_current_event(event);
    let mut window = crate::nsevent::event_window(event);
    if window == 0 {
        window = current_key_window();
    }
    if window == 0 {
        window = current_main_window();
    }

    if window != 0 {
        if crate::nsevent::event_window(event) == 0 {
            crate::nsevent::set_event_window(event, window);
        }
        crate::nswindow::shim_nswindow_send_event(window, 0, event);
        restore_previous_current_event(previous_event, event);
        return;
    }

    if let Some(selector) = crate::nsevent::event_dispatch_selector(event) {
        let _ = dispatch_selector_named_to_target(obj, selector, event);
    }

    restore_previous_current_event(previous_event, event);
}

/// `-[NSApplication isRunning]`
pub extern "C" fn shim_nsapplication_is_running(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let val = unsafe { *((obj as usize + IS_RUNNING_OFF) as *const u64) };
    u8::from(val != 0)
}

/// `-[NSApplication run]`
///
/// In a real app this enters the event loop indefinitely. For our shim
/// runtime this does a minimal runloop: calls applicationDidFinishLaunching:
/// on the delegate, runs one iteration, then returns (since we can't block
/// a mapped binary's thread without a real event source).
pub extern "C" fn shim_nsapplication_run(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }

    unsafe { *((obj as usize + IS_RUNNING_OFF) as *mut u64) = 1; }
    log::debug!("NSApplication run: entering run loop");

    let delegate = unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) };
    if delegate != 0 {
        let note = create_launch_notification();
        send_delegate_message(delegate, "applicationDidFinishLaunching:", note);
    }

    log::debug!("NSApplication run: run loop iteration complete");
}

/// `-[NSApplication terminate:]`
pub extern "C" fn shim_nsapplication_terminate(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }

    let delegate = unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) };
    if delegate != 0 {
        send_delegate_message(delegate, "applicationWillTerminate:", 0);
    }

    unsafe { *((obj as usize + IS_RUNNING_OFF) as *mut u64) = 0; }
    log::debug!("NSApplication terminate: stopped");
}

/// `-[NSApplication activateIgnoringOtherApps:]` — stub, no-op.
pub extern "C" fn shim_nsapplication_activate(_obj: u64, _cmd: u64, _flag: u8) {
    log::debug!("NSApplication activateIgnoringOtherApps: (stub)");
}

/// `-[NSApplication finishLaunching]` — stub, no-op.
pub extern "C" fn shim_nsapplication_finish_launching(_obj: u64, _cmd: u64) {
    log::debug!("NSApplication finishLaunching (stub)");
}

/// `-[NSApplication setActivationPolicy:]` — stub, returns YES.
pub extern "C" fn shim_nsapplication_set_activation_policy(_obj: u64, _cmd: u64, _policy: i64) -> u8 {
    log::debug!("NSApplication setActivationPolicy: (stub)");
    1
}

/// `-[NSApplication windows]` — returns registered guest windows.
pub extern "C" fn shim_nsapplication_windows(_obj: u64, _cmd: u64) -> u64 {
    let windows = snapshot_windows();
    crate::nsarray::make_nsarray(&windows)
}

/// `-[NSApplication orderedWindows]` — returns visible guest windows from front to back.
pub extern "C" fn shim_nsapplication_ordered_windows(_obj: u64, _cmd: u64) -> u64 {
    let windows = snapshot_ordered_windows();
    crate::nsarray::make_nsarray(&windows)
}

/// `-[NSApplication sendAction:to:from:]`
pub extern "C" fn shim_nsapplication_send_action_to_from(
    obj: u64,
    _cmd: u64,
    action: u64,
    target: u64,
    sender: u64,
) -> u8 {
    if obj == 0 || action == 0 {
        return 0;
    }

    let resolved = resolved_action_target_for_selector(action, target);
    if resolved == 0 {
        return 0;
    }

    u8::from(dispatch_selector_ptr_to_target(action, resolved, sender))
}

// ── Helpers ────────────────────────────────────────────────────────

fn receiver_is_kind_of(obj: u64, class_addr: u64) -> bool {
    if obj == 0 || class_addr == 0 {
        return false;
    }

    let isa = unsafe { *(obj as *const u64) };
    if isa == class_addr {
        return true;
    }

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let mut current = Some(isa);
    let mut depth = 0;
    while let Some(addr) = current {
        if depth > 64 {
            break;
        }
        if addr == class_addr {
            return true;
        }
        current = rt.get_class_by_addr(addr).and_then(|class| class.superclass_addr);
        depth += 1;
    }

    false
}

pub(crate) fn dispatch_selector_ptr_to_target(sel: u64, target: u64, arg: u64) -> bool {
    if sel == 0 || target == 0 {
        return false;
    }

    let sel_name = unsafe {
        std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };
    if sel_name.is_empty() {
        return false;
    }

    let isa = unsafe { *(target as *const u64) };

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let Some(imp) = rt.msg_lookup(isa, sel_name) else {
        return false;
    };

    let sel_ptr = rt.selector_ptr(sel_name).unwrap_or(sel as usize) as u64;
    drop(guard);

    let func: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, arg);
    true
}

pub(crate) fn target_responds_to_selector_ptr(sel: u64, target: u64) -> bool {
    if sel == 0 || target == 0 {
        return false;
    }

    let sel_name = unsafe {
        std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };
    if sel_name.is_empty() {
        return false;
    }

    let isa = unsafe { *(target as *const u64) };

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    rt.msg_lookup(isa, sel_name).is_some()
}

pub(crate) fn dispatch_selector_named_to_target(target: u64, selector: &str, arg: u64) -> bool {
    if target == 0 || selector.is_empty() {
        return false;
    }

    let isa = unsafe { *(target as *const u64) };

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let Some(imp) = rt.msg_lookup(isa, selector) else {
        return false;
    };

    let sel_ptr = rt.selector_ptr(selector).unwrap_or(0) as u64;
    drop(guard);

    let func: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, arg);
    true
}

/// Dispatch a BOOL-returning selector with one argument through the ObjC runtime.
/// Returns `false` when the runtime or method is unavailable.
pub(crate) fn dispatch_bool_selector_named_to_target(target: u64, selector: &str, arg: u64) -> bool {
    if target == 0 || selector.is_empty() {
        return false;
    }

    let isa = unsafe { *(target as *const u64) };

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let Some(imp) = rt.msg_lookup(isa, selector) else {
        return false;
    };

    let sel_ptr = rt.selector_ptr(selector).unwrap_or(0) as u64;
    drop(guard);

    let func: extern "C" fn(u64, u64, u64) -> u8 = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, arg) != 0
}

fn push_unique_target(targets: &mut Vec<u64>, target: u64) {
    if target != 0 && !targets.contains(&target) {
        targets.push(target);
    }
}

fn next_action_target(current: u64, window: u64) -> u64 {
    if current == 0 {
        return 0;
    }

    if receiver_is_kind_of(current, crate::nsobj::nsview_class_addr()) {
        let superview = crate::nsview::shim_nsview_superview(current, 0);
        if superview != 0 {
            return superview;
        }
        let owning_window = crate::nsview::shim_nsview_window(current, 0);
        if owning_window != 0 && owning_window != current {
            return owning_window;
        }
        return 0;
    }

    if receiver_is_kind_of(current, crate::nsobj::nswindow_class_addr()) {
        return 0;
    }

    let next = crate::nsresponder::shim_nsresponder_next_responder(current, 0);
    if next != 0 {
        return next;
    }

    if window != 0 && current != window {
        return window;
    }

    0
}

fn append_window_action_targets(targets: &mut Vec<u64>, window: u64) {
    if window == 0 {
        return;
    }

    let mut current = crate::nswindow::shim_nswindow_first_responder(window, 0);
    let mut depth = 0;
    while current != 0 && depth < 64 {
        push_unique_target(targets, current);
        let next = next_action_target(current, window);
        if next == 0 || next == current {
            break;
        }
        current = next;
        depth += 1;
    }

    push_unique_target(targets, window);
}

fn resolved_action_targets(explicit_target: u64) -> Vec<u64> {
    if explicit_target != 0 {
        return vec![explicit_target];
    }

    let mut targets = Vec::new();
    let app = get_or_create_shared_app();
    let key_window = current_key_window();
    let main_window = current_main_window();

    append_window_action_targets(&mut targets, key_window);
    if main_window != key_window {
        append_window_action_targets(&mut targets, main_window);
    }

    push_unique_target(&mut targets, shim_nsapplication_delegate(app, 0));
    push_unique_target(&mut targets, app);
    targets
}

fn resolved_action_target_for_selector(action: u64, explicit_target: u64) -> u64 {
    resolved_action_targets(explicit_target)
        .into_iter()
        .find(|&candidate| target_responds_to_selector_ptr(action, candidate))
        .unwrap_or(0)
}

fn create_launch_notification() -> u64 {
    let name_cstr = std::ffi::CString::new("NSApplicationDidFinishLaunchingNotification").unwrap();
    crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr())
}

fn send_delegate_message(delegate: u64, selector: &str, arg: u64) {
    if !dispatch_selector_named_to_target(delegate, selector, arg) {
        log::debug!(
            "NSApplication: delegate {:#x} does not respond to {}",
            delegate,
            selector
        );
    }
}

/// `-[NSApplication nextEventMatchingMask:untilDate:inMode:dequeue:]`
/// Stub — returns nil (no real event loop).
pub extern "C" fn shim_nsapplication_next_event(
    _obj: u64, _cmd: u64,
    _mask: u64, _until_date: u64, _mode: u64, _dequeue: u8,
) -> u64 {
    0
}

/// `-[NSApplication performKeyEquivalent:]`
pub extern "C" fn shim_nsapplication_perform_key_equivalent(obj: u64, _cmd: u64, event: u64) -> u8 {
    if obj == 0 || event == 0 {
        return 0;
    }

    let key_window = current_key_window();
    if key_window != 0
        && dispatch_bool_selector_named_to_target(key_window, "performKeyEquivalent:", event)
    {
        return 1;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    const SIZE: usize = APP_SIZE;

    static APP_TEST_LOCK: Mutex<()> = Mutex::new(());
    static ACTION_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_ACTION_TARGET: AtomicU64 = AtomicU64::new(0);
    static LAST_ACTION_SENDER: AtomicU64 = AtomicU64::new(0);
    static EVENT_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_TARGET: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_ARG: AtomicU64 = AtomicU64::new(0);
    static CURRENT_EVENT_DURING_EVENT_CALL: AtomicU64 = AtomicU64::new(0);

    const ACTION_PROBE_CLASS_ADDR: u64 = 0x0000_0001_0050_1000;

    extern "C" fn action_probe_imp(obj: u64, _cmd: u64, sender: u64) {
        ACTION_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        LAST_ACTION_TARGET.store(obj, Ordering::SeqCst);
        LAST_ACTION_SENDER.store(sender, Ordering::SeqCst);
    }

    extern "C" fn event_probe_imp(obj: u64, _cmd: u64, event: u64) {
        EVENT_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        LAST_EVENT_TARGET.store(obj, Ordering::SeqCst);
        LAST_EVENT_ARG.store(event, Ordering::SeqCst);
        let app = shim_nsapplication_shared_application(0, 0);
        CURRENT_EVENT_DURING_EVENT_CALL.store(
            shim_nsapplication_current_event(app, 0),
            Ordering::SeqCst,
        );
    }

    fn make_app() -> u64 {
        alloc_obj(SIZE, crate::nsobj::nsapplication_class_addr())
    }

    fn reset_action_probe() {
        ACTION_CALL_COUNT.store(0, Ordering::SeqCst);
        LAST_ACTION_TARGET.store(0, Ordering::SeqCst);
        LAST_ACTION_SENDER.store(0, Ordering::SeqCst);
        EVENT_CALL_COUNT.store(0, Ordering::SeqCst);
        LAST_EVENT_TARGET.store(0, Ordering::SeqCst);
        LAST_EVENT_ARG.store(0, Ordering::SeqCst);
        CURRENT_EVENT_DURING_EVENT_CALL.store(0, Ordering::SeqCst);
    }

    fn install_action_probe_class(rt: &mut objc_runtime::ObjcRuntime) {
        let mut info = objc_runtime::ClassInfo {
            name: "ActionProbe".to_string(),
            mapped_addr: ACTION_PROBE_CLASS_ADDR,
            methods: vec![objc_runtime::MethodEntry {
                selector: "performAction:".into(),
                imp: action_probe_imp as *const () as u64,
                type_encoding: "v@:@".into(),
            }, objc_runtime::MethodEntry {
                selector: "keyDown:".into(),
                imp: event_probe_imp as *const () as u64,
                type_encoding: "v@:@".into(),
            }],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 16,
            is_root: false,
            is_meta: false,
            superclass_addr: Some(crate::nsobj::nsresponder_class_addr()),
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    fn with_action_runtime<R>(f: impl FnOnce() -> R) -> R {
        let saved_runtime = {
            let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap_or_else(|e| e.into_inner());
            let saved = std::mem::take(&mut *guard);

            let mut rt = objc_runtime::ObjcRuntime::new();
            crate::FoundationShims::register_nsobject_class(&mut rt);
            crate::FoundationShims::register_nsresponder_class(&mut rt);
            crate::FoundationShims::register_nsapplication_class(&mut rt);
            crate::FoundationShims::register_nswindow_class(&mut rt);
            crate::FoundationShims::register_nsview_class(&mut rt);
            crate::FoundationShims::register_nsevent_class(&mut rt);
            install_action_probe_class(&mut rt);

            *guard = Some(rt);
            saved
        };

        let result = f();

        let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap_or_else(|e| e.into_inner());
        *guard = saved_runtime;
        result
    }

    fn intern_selector(name: &str) -> u64 {
        let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap_or_else(|e| e.into_inner());
        guard
            .as_mut()
            .expect("test ObjC runtime should be installed")
            .intern_selector(name) as u64
    }

    #[test]
    fn shared_application_returns_nonzero() {
        let app = shim_nsapplication_shared_application(0, 0);
        assert_ne!(app, 0);
    }

    #[test]
    fn shared_application_is_idempotent() {
        let a = shim_nsapplication_shared_application(0, 0);
        let b = shim_nsapplication_shared_application(0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn nsapp_global_addr_returns_nonzero() {
        let addr = nsapp_global_addr();
        assert_ne!(addr, 0);
        let app = unsafe { *(addr as *const u64) };
        assert_ne!(app, 0);
    }

    #[test]
    fn delegate_nil_returns_zero() {
        assert_eq!(shim_nsapplication_delegate(0, 0), 0);
    }

    #[test]
    fn delegate_default_is_zero() {
        let app = make_app();
        assert_eq!(shim_nsapplication_delegate(app, 0), 0);
        free_obj(app);
    }

    #[test]
    fn delegate_roundtrip() {
        let app = make_app();
        shim_nsapplication_set_delegate(app, 0, 0xCAFE);
        assert_eq!(shim_nsapplication_delegate(app, 0), 0xCAFE);
        free_obj(app);
    }

    #[test]
    fn set_delegate_nil_no_crash() {
        shim_nsapplication_set_delegate(0, 0, 0xABC);
    }

    #[test]
    fn main_menu_nil_returns_zero() {
        assert_eq!(shim_nsapplication_main_menu(0, 0), 0);
    }

    #[test]
    fn main_menu_default_is_zero() {
        let app = make_app();
        assert_eq!(shim_nsapplication_main_menu(app, 0), 0);
        free_obj(app);
    }

    #[test]
    fn main_menu_roundtrip() {
        let app = make_app();
        shim_nsapplication_set_main_menu(app, 0, 0xBEEF);
        assert_eq!(shim_nsapplication_main_menu(app, 0), 0xBEEF);
        free_obj(app);
    }

    #[test]
    fn set_main_menu_nil_no_crash() {
        shim_nsapplication_set_main_menu(0, 0, 0xBEEF);
    }

    #[test]
    fn key_window_nil_returns_zero() {
        assert_eq!(shim_nsapplication_key_window(0, 0), 0);
    }

    #[test]
    fn main_window_nil_returns_zero() {
        assert_eq!(shim_nsapplication_main_window(0, 0), 0);
    }

    #[test]
    fn key_and_main_window_reflect_fields() {
        let app = make_app();
        write_field(app, KEY_WINDOW_OFF, 0xAAAA);
        write_field(app, MAIN_WINDOW_OFF, 0xBBBB);
        assert_eq!(shim_nsapplication_key_window(app, 0), 0xAAAA);
        assert_eq!(shim_nsapplication_main_window(app, 0), 0xBBBB);
        free_obj(app);
    }

    #[test]
    fn current_event_nil_returns_zero() {
        assert_eq!(shim_nsapplication_current_event(0, 0), 0);
    }

    #[test]
    fn current_event_tracks_shared_state() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        let app = shim_nsapplication_shared_application(0, 0);
        assert_eq!(shim_nsapplication_current_event(app, 0), 0);

        set_current_event(0xC0DE);
        assert_eq!(shim_nsapplication_current_event(app, 0), 0xC0DE);

        reset_runtime_state_for_reuse();
        assert_eq!(shim_nsapplication_current_event(app, 0), 0);
    }

    #[test]
    fn target_for_action_with_nil_target_walks_first_responder_chain() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");
            let window = crate::nswindow::shim_nswindow_init_with_content_rect(
                0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
            );
            let first = alloc_obj(16, crate::nsobj::nsresponder_class_addr());
            let target = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);

            crate::nsresponder::shim_nsresponder_set_next_responder(first, 0, target);
            crate::nswindow::shim_nswindow_make_key_and_order_front(window, 0, 0);
            assert_eq!(crate::nswindow::shim_nswindow_make_first_responder(window, 0, first), 1);

            assert_eq!(shim_nsapplication_target_for_action(app, 0, action), target);

            free_obj(target);
            free_obj(first);
            free_obj(window);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn target_for_action_to_from_with_explicit_target_returns_target() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");
            let target = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);

            assert_eq!(
                shim_nsapplication_target_for_action_to_from(app, 0, action, target, 0xABCD),
                target
            );

            free_obj(target);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn target_for_action_to_from_returns_zero_for_unhandled_explicit_target() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");
            let target = alloc_obj(16, crate::nsobj::nsresponder_class_addr());

            assert_eq!(
                shim_nsapplication_target_for_action_to_from(app, 0, action, target, 0xABCD),
                0
            );

            free_obj(target);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn target_for_action_with_nil_target_falls_back_to_delegate() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");
            let delegate = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);

            shim_nsapplication_set_delegate(app, 0, delegate);

            assert_eq!(shim_nsapplication_target_for_action(app, 0, action), delegate);

            free_obj(delegate);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn target_for_action_returns_zero_when_unhandled() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");

            assert_eq!(shim_nsapplication_target_for_action(app, 0, action), 0);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn send_event_dispatches_to_key_window_first_responder() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        reset_action_probe();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let window = crate::nswindow::shim_nswindow_init_with_content_rect(
                0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
            );
            let target = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            write_field(event, 8, 10);
            crate::nswindow::shim_nswindow_make_key_and_order_front(window, 0, 0);
            assert_eq!(crate::nswindow::shim_nswindow_make_first_responder(window, 0, target), 1);

            shim_nsapplication_send_event(app, 0, event);

            assert_eq!(EVENT_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_EVENT_TARGET.load(Ordering::SeqCst), target);
            assert_eq!(LAST_EVENT_ARG.load(Ordering::SeqCst), event);
            assert_eq!(CURRENT_EVENT_DURING_EVENT_CALL.load(Ordering::SeqCst), event);
            assert_eq!(crate::nsevent::shim_nsevent_window(event, 0), window);
            assert_eq!(shim_nsapplication_current_event(app, 0), event);

            free_obj(event);
            free_obj(target);
            free_obj(window);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn send_event_prefers_explicit_event_window_over_key_window() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        reset_action_probe();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let first_window = crate::nswindow::shim_nswindow_init_with_content_rect(
                0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
            );
            let second_window = crate::nswindow::shim_nswindow_init_with_content_rect(
                0, 0, 30.0, 40.0, 320.0, 240.0, 1, 2, 0,
            );
            let first_target = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);
            let second_target = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            write_field(event, 8, 10);
            crate::nsevent::set_event_window(event, first_window);
            assert_eq!(crate::nswindow::shim_nswindow_make_first_responder(first_window, 0, first_target), 1);
            crate::nswindow::shim_nswindow_make_key_and_order_front(second_window, 0, 0);
            assert_eq!(crate::nswindow::shim_nswindow_make_first_responder(second_window, 0, second_target), 1);

            shim_nsapplication_send_event(app, 0, event);

            assert_eq!(EVENT_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_EVENT_TARGET.load(Ordering::SeqCst), first_target);
            assert_eq!(crate::nsevent::shim_nsevent_window(event, 0), first_window);

            free_obj(event);
            free_obj(second_target);
            free_obj(first_target);
            free_obj(second_window);
            free_obj(first_window);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn push_current_event_restores_previous_nested_event() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        let app = shim_nsapplication_shared_application(0, 0);
        set_current_event(0x1111);

        let previous = push_current_event(0x2222);
        assert_eq!(previous, 0x1111);
        assert_eq!(shim_nsapplication_current_event(app, 0), 0x2222);

        restore_previous_current_event(previous, 0x2222);
        assert_eq!(shim_nsapplication_current_event(app, 0), 0x1111);

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn push_current_event_keeps_top_level_event_after_dispatch() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        let app = shim_nsapplication_shared_application(0, 0);
        let previous = push_current_event(0x3333);
        assert_eq!(previous, 0);
        assert_eq!(shim_nsapplication_current_event(app, 0), 0x3333);

        restore_previous_current_event(previous, 0x3333);
        assert_eq!(shim_nsapplication_current_event(app, 0), 0x3333);

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn is_running_nil_returns_zero() {
        assert_eq!(shim_nsapplication_is_running(0, 0), 0);
    }

    #[test]
    fn is_running_default_is_zero() {
        let app = make_app();
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        free_obj(app);
    }

    #[test]
    fn is_running_reflects_field() {
        let app = make_app();
        write_field(app, IS_RUNNING_OFF, 1);
        assert_eq!(shim_nsapplication_is_running(app, 0), 1);
        write_field(app, IS_RUNNING_OFF, 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        free_obj(app);
    }

    #[test]
    fn is_running_nonzero_yields_one() {
        let app = make_app();
        write_field(app, IS_RUNNING_OFF, 999);
        assert_eq!(shim_nsapplication_is_running(app, 0), 1);
        free_obj(app);
    }

    #[test]
    fn run_nil_no_crash() {
        shim_nsapplication_run(0, 0);
    }

    #[test]
    fn run_sets_is_running() {
        let app = make_app();
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        shim_nsapplication_run(app, 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 1);
        free_obj(app);
    }

    #[test]
    fn terminate_nil_no_crash() {
        shim_nsapplication_terminate(0, 0, 0);
    }

    #[test]
    fn terminate_clears_is_running() {
        let app = make_app();
        write_field(app, IS_RUNNING_OFF, 1);
        assert_eq!(shim_nsapplication_is_running(app, 0), 1);
        shim_nsapplication_terminate(app, 0, 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        free_obj(app);
    }

    #[test]
    fn activate_nil_no_crash() {
        shim_nsapplication_activate(0, 0, 1);
    }

    #[test]
    fn activate_no_side_effect() {
        let app = make_app();
        shim_nsapplication_activate(app, 0, 1);
        assert_eq!(shim_nsapplication_delegate(app, 0), 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        free_obj(app);
    }

    #[test]
    fn finish_launching_nil_no_crash() {
        shim_nsapplication_finish_launching(0, 0);
    }

    #[test]
    fn set_activation_policy_nil_returns_one() {
        assert_eq!(shim_nsapplication_set_activation_policy(0, 0, 0), 1);
    }

    #[test]
    fn set_activation_policy_always_returns_one() {
        let app = make_app();
        assert_eq!(shim_nsapplication_set_activation_policy(app, 0, 0), 1);
        assert_eq!(shim_nsapplication_set_activation_policy(app, 0, 1), 1);
        assert_eq!(shim_nsapplication_set_activation_policy(app, 0, -1), 1);
        free_obj(app);
    }

    #[test]
    fn windows_initially_empty() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        let app = shim_nsapplication_shared_application(0, 0);
        let arr = shim_nsapplication_windows(app, 0);
        assert_ne!(arr, 0, "windows should return an allocated NSArray");
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
    }

    #[test]
    fn windows_track_initialized_window() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        let app = shim_nsapplication_shared_application(0, 0);
        let window = crate::nswindow::shim_nswindow_init_with_content_rect(
            0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
        );
        assert_ne!(window, 0);

        let arr = shim_nsapplication_windows(app, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0), window);

        reset_runtime_state_for_reuse();
        let cleared = shim_nsapplication_windows(app, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(cleared, 0), 0);
        free_obj(window);
    }

    #[test]
    fn ordered_windows_initially_empty() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        let app = shim_nsapplication_shared_application(0, 0);
        let arr = shim_nsapplication_ordered_windows(app, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
    }

    #[test]
    fn ordered_windows_are_front_to_back_and_visible_only() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        let app = shim_nsapplication_shared_application(0, 0);
        let first = crate::nswindow::shim_nswindow_init_with_content_rect(
            0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
        );
        let second = crate::nswindow::shim_nswindow_init_with_content_rect(
            0, 0, 30.0, 40.0, 320.0, 240.0, 1, 2, 0,
        );

        crate::nswindow::shim_nswindow_order_front(first, 0, 0);
        crate::nswindow::shim_nswindow_order_front(second, 0, 0);

        let ordered = shim_nsapplication_ordered_windows(app, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(ordered, 0), 2);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 0), second);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 1), first);

        crate::nswindow::shim_nswindow_order_out(second, 0, 0);
        let visible_only = shim_nsapplication_ordered_windows(app, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(visible_only, 0), 1);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(visible_only, 0, 0), first);

        crate::nswindow::shim_nswindow_order_front(second, 0, 0);
        crate::nswindow::shim_nswindow_order_front(first, 0, 0);
        let reordered = shim_nsapplication_ordered_windows(app, 0);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(reordered, 0, 0), first);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(reordered, 0, 1), second);

        free_obj(second);
        free_obj(first);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn send_action_with_explicit_target_dispatches() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        reset_action_probe();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");
            let target = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);
            let sender = 0xABCD;

            assert_eq!(shim_nsapplication_send_action_to_from(app, 0, action, target, sender), 1);
            assert_eq!(ACTION_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_ACTION_TARGET.load(Ordering::SeqCst), target);
            assert_eq!(LAST_ACTION_SENDER.load(Ordering::SeqCst), sender);

            free_obj(target);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn send_action_with_nil_target_walks_first_responder_chain() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        reset_action_probe();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");
            let window = crate::nswindow::shim_nswindow_init_with_content_rect(
                0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
            );
            let first = alloc_obj(16, crate::nsobj::nsresponder_class_addr());
            let target = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);
            let sender = 0xBCDE;

            crate::nsresponder::shim_nsresponder_set_next_responder(first, 0, target);
            crate::nswindow::shim_nswindow_make_key_and_order_front(window, 0, 0);
            assert_eq!(crate::nswindow::shim_nswindow_make_first_responder(window, 0, first), 1);

            assert_eq!(shim_nsapplication_send_action_to_from(app, 0, action, 0, sender), 1);
            assert_eq!(ACTION_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_ACTION_TARGET.load(Ordering::SeqCst), target);
            assert_eq!(LAST_ACTION_SENDER.load(Ordering::SeqCst), sender);

            free_obj(target);
            free_obj(first);
            free_obj(window);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn send_action_with_nil_target_falls_back_to_delegate() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        reset_action_probe();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");
            let delegate = alloc_obj(16, ACTION_PROBE_CLASS_ADDR);
            let sender = 0xCDEF;

            shim_nsapplication_set_delegate(app, 0, delegate);

            assert_eq!(shim_nsapplication_send_action_to_from(app, 0, action, 0, sender), 1);
            assert_eq!(ACTION_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_ACTION_TARGET.load(Ordering::SeqCst), delegate);
            assert_eq!(LAST_ACTION_SENDER.load(Ordering::SeqCst), sender);

            free_obj(delegate);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn send_action_returns_no_when_unhandled() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        reset_action_probe();

        with_action_runtime(|| {
            let app = shim_nsapplication_shared_application(0, 0);
            let action = intern_selector("performAction:");

            assert_eq!(shim_nsapplication_send_action_to_from(app, 0, action, 0, 0xDEAD), 0);
            assert_eq!(ACTION_CALL_COUNT.load(Ordering::SeqCst), 0);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn reset_runtime_state_clears_shared_app_fields() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        let app = shim_nsapplication_shared_application(0, 0);
        shim_nsapplication_set_delegate(app, 0, 0xCAFE);
        shim_nsapplication_set_main_menu(app, 0, 0xBEEF);
        set_key_window(0xAAAA);
        set_main_window(0xBBBB);
        set_current_event(0xCCCC);
        unsafe {
            *((app as usize + IS_RUNNING_OFF) as *mut u64) = 1;
        }

        let window = crate::nswindow::shim_nswindow_init_with_content_rect(
            0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0,
        );
        assert_ne!(window, 0);

        reset_runtime_state_for_reuse();

        assert_eq!(shim_nsapplication_delegate(app, 0), 0);
        assert_eq!(shim_nsapplication_main_menu(app, 0), 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        assert_eq!(shim_nsapplication_key_window(app, 0), 0);
        assert_eq!(shim_nsapplication_main_window(app, 0), 0);
        assert_eq!(shim_nsapplication_current_event(app, 0), 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(shim_nsapplication_windows(app, 0), 0), 0);
        free_obj(window);
    }

    #[test]
    fn run_then_terminate_cycle() {
        let app = make_app();
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        shim_nsapplication_run(app, 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 1);
        shim_nsapplication_terminate(app, 0, 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        free_obj(app);
    }

    // ---- performKeyEquivalent ----

    #[test]
    fn perform_key_equivalent_nil_returns_no() {
        assert_eq!(shim_nsapplication_perform_key_equivalent(0, 0, 0), 0);
    }

    #[test]
    fn perform_key_equivalent_no_key_window_returns_no() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        let app = shim_nsapplication_shared_application(0, 0);
        let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

        assert_eq!(shim_nsapplication_perform_key_equivalent(app, 0, event), 0);

        free_obj(event);
        reset_runtime_state_for_reuse();
    }

    // ---- nextEventMatchingMask ----

    #[test]
    fn next_event_returns_nil() {
        let app = make_app();
        assert_eq!(shim_nsapplication_next_event(app, 0, 0xFFFF_FFFF, 0, 0, 1), 0);
        free_obj(app);
    }

    #[test]
    fn next_event_nil_does_not_crash() {
        assert_eq!(shim_nsapplication_next_event(0, 0, 0, 0, 0, 0), 0);
    }
}
