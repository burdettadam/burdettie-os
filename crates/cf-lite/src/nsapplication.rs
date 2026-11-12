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
//!   offset 56: windows_menu_ptr (u64 — NSMenu object, or 0)
//!   offset 64: help_menu_ptr    (u64 — NSMenu object, or 0)
//!   offset 72: finished_launching (u64 — 1 once launch notifications were sent)
//!
//! Total: 80 bytes.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

const DELEGATE_OFF: usize = 8;
const MAIN_MENU_OFF: usize = 16;
const IS_RUNNING_OFF: usize = 24;
const KEY_WINDOW_OFF: usize = 32;
const MAIN_WINDOW_OFF: usize = 40;
const CURRENT_EVENT_OFF: usize = 48;
const WINDOWS_MENU_OFF: usize = 56;
const HELP_MENU_OFF: usize = 64;
const FINISHED_LAUNCHING_OFF: usize = 72;
const APP_SIZE: usize = 80;

static SHARED_APP: OnceLock<u64> = OnceLock::new();
static PRINCIPAL_APP_CLASS: OnceLock<Mutex<u64>> = OnceLock::new();
static APP_WINDOWS: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();
static EVENT_QUEUE: OnceLock<Mutex<VecDeque<u64>>> = OnceLock::new();

fn principal_app_class() -> &'static Mutex<u64> {
    PRINCIPAL_APP_CLASS.get_or_init(|| Mutex::new(crate::nsobj::nsapplication_class_addr()))
}

fn event_queue() -> &'static Mutex<VecDeque<u64>> {
    EVENT_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Push an NSEvent address onto the application event queue.
pub(crate) fn enqueue_event(event: u64) {
    if event == 0 {
        return;
    }
    event_queue()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push_back(event);
}

/// Dequeue the first event whose type matches `mask`, or any event if mask is
/// `NSEventMaskAny` (all bits set / `u64::MAX`). Returns 0 if no matching event.
fn dequeue_event(mask: u64, dequeue: bool) -> u64 {
    let mut queue = event_queue().lock().unwrap_or_else(|e| e.into_inner());
    if queue.is_empty() {
        return 0;
    }
    // NSEventMaskAny = NSUIntegerMax = all bits set
    let match_any = mask == u64::MAX || mask == 0;
    for i in 0..queue.len() {
        let event = queue[i];
        if match_any || event_matches_mask(event, mask) {
            if dequeue {
                queue.remove(i);
            }
            return event;
        }
    }
    0
}

/// Check if an NSEvent's type matches the given event mask.
fn event_matches_mask(event: u64, mask: u64) -> bool {
    if event == 0 {
        return false;
    }
    let event_type = crate::nsevent::event_type_raw(event);
    if event_type >= 64 {
        return false; // invalid type
    }
    (mask & (1u64 << event_type)) != 0
}

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
    app_windows()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
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
    if let Some(class_slot) = PRINCIPAL_APP_CLASS.get() {
        *class_slot.lock().unwrap_or_else(|e| e.into_inner()) =
            crate::nsobj::nsapplication_class_addr();
    }
    clear_registered_windows();
    event_queue()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    if let Some(app) = SHARED_APP.get().copied() {
        unsafe {
            *(app as *mut u64) = crate::nsobj::nsapplication_class_addr();
            *((app as usize + DELEGATE_OFF) as *mut u64) = 0;
            *((app as usize + MAIN_MENU_OFF) as *mut u64) = 0;
            *((app as usize + IS_RUNNING_OFF) as *mut u64) = 0;
            *((app as usize + KEY_WINDOW_OFF) as *mut u64) = 0;
            *((app as usize + MAIN_WINDOW_OFF) as *mut u64) = 0;
            *((app as usize + CURRENT_EVENT_OFF) as *mut u64) = 0;
            *((app as usize + WINDOWS_MENU_OFF) as *mut u64) = 0;
            *((app as usize + HELP_MENU_OFF) as *mut u64) = 0;
            *((app as usize + FINISHED_LAUNCHING_OFF) as *mut u64) = 0;
        }
    }
}

fn configured_application_class_addr() -> u64 {
    let class_addr = *principal_app_class()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if class_addr == 0 {
        crate::nsobj::nsapplication_class_addr()
    } else {
        class_addr
    }
}

fn application_instance_size(class_addr: u64) -> usize {
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    guard
        .as_ref()
        .and_then(|rt| rt.get_class_by_addr(class_addr))
        .map(|class_info| class_info.instance_size as usize)
        .unwrap_or(APP_SIZE)
        .max(APP_SIZE)
}

fn update_nsapp_global_slot(app: u64) {
    if let Some(slot) = NSAPP_GLOBAL.get().copied() {
        unsafe {
            *(slot as *mut u64) = app;
        }
    }
}

fn set_principal_application_class(class_addr: u64) {
    if class_addr == 0 {
        return;
    }

    if let Some(app) = SHARED_APP.get().copied() {
        let size = application_instance_size(class_addr);
        if size > APP_SIZE {
            log::warn!(
                "NSApplicationMain: principal class requires {} bytes, but shared app already exists with base layout",
                size
            );
            return;
        }
        unsafe {
            *(app as *mut u64) = class_addr;
        }
        update_nsapp_global_slot(app);
    }

    *principal_app_class()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = class_addr;
}

fn get_or_create_shared_app() -> u64 {
    *SHARED_APP.get_or_init(|| {
        let class_addr = configured_application_class_addr();
        let obj = crate::pal_alloc().calloc(1, application_instance_size(class_addr)) as u64;
        if obj == 0 {
            return 0;
        }
        unsafe {
            *(obj as *mut u64) = class_addr;
        }
        update_nsapp_global_slot(obj);
        log::debug!(
            "NSApplication sharedApplication: created singleton at {:#x}",
            obj
        );
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
    unsafe {
        *((obj as usize + DELEGATE_OFF) as *mut u64) = delegate;
    }
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
    unsafe {
        *((obj as usize + MAIN_MENU_OFF) as *mut u64) = menu;
    }
}

/// `-[NSApplication windowsMenu]`
pub extern "C" fn shim_nsapplication_windows_menu(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + WINDOWS_MENU_OFF) as *const u64) }
}

/// `-[NSApplication setWindowsMenu:]`
pub extern "C" fn shim_nsapplication_set_windows_menu(obj: u64, _cmd: u64, menu: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + WINDOWS_MENU_OFF) as *mut u64) = menu;
    }
}

/// `-[NSApplication helpMenu]`
pub extern "C" fn shim_nsapplication_help_menu(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + HELP_MENU_OFF) as *const u64) }
}

/// `-[NSApplication setHelpMenu:]`
pub extern "C" fn shim_nsapplication_set_help_menu(obj: u64, _cmd: u64, menu: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + HELP_MENU_OFF) as *mut u64) = menu;
    }
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

pub(crate) fn set_app_running(is_running: bool) {
    let app = get_or_create_shared_app();
    if app == 0 {
        return;
    }

    unsafe {
        *((app as usize + IS_RUNNING_OFF) as *mut u64) = u64::from(is_running);
    }
}

pub(crate) fn run_gui_event_loop_if_available() -> bool {
    let Some(bridge) = crate::gui_bridge::gui_bridge() else {
        return false;
    };

    log::info!("GUI bridge: entering event loop");

    for &window in &snapshot_windows() {
        if crate::nswindow::is_window_visible(window) {
            let desc = crate::nswindow::window_desc(window);
            bridge.show_window(&desc);
        }
    }

    bridge.run_event_loop(Box::new(|guest_window, event| match event {
        crate::gui_bridge::GuiEvent::WindowCloseRequested => {
            if guest_window != 0 {
                crate::nswindow::shim_nswindow_perform_close(guest_window, 0, 0);
            }
        }
        crate::gui_bridge::GuiEvent::AppTerminate => {
            shim_nsapplication_terminate(get_or_create_shared_app(), 0, 0);
        }
        crate::gui_bridge::GuiEvent::WindowResized { width, height } => {
            if guest_window != 0 {
                crate::nswindow::shim_nswindow_set_frame_display(
                    guest_window,
                    0,
                    0.0,
                    0.0,
                    width,
                    height,
                    1,
                );
            }
        }
        other => {
            crate::gui_bridge::dispatch_gui_event(guest_window, &other);
        }
    }));

    log::info!("GUI bridge: event loop exited");
    true
}

/// `-[NSApplication run]`
///
/// In a real app this enters the event loop indefinitely. For our shim
/// runtime this dispatches launch notifications once, then enters the GUI
/// bridge if available or returns after a single headless pass.
pub extern "C" fn shim_nsapplication_run(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }

    set_app_running(true);
    log::debug!("NSApplication run: entering run loop");
    shim_nsapplication_finish_launching(obj, 0);

    if run_gui_event_loop_if_available() {
        set_app_running(false);
        return;
    }

    // No bridge — run one cycle and return (headless/CLI behavior).
    log::debug!("NSApplication run: no GUI bridge, single-pass complete");
    set_app_running(false);
}

/// `-[NSApplication terminate:]`
///
/// Queries the delegate with `applicationShouldTerminate:` before actually
/// terminating.  If the delegate returns `NSTerminateCancel` (0) the
/// termination is aborted.  `NSTerminateNow` (1) or no delegate proceeds.
pub extern "C" fn shim_nsapplication_terminate(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }

    let delegate = unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) };

    // Ask delegate: applicationShouldTerminate: → NSTerminateCancel(0) / NSTerminateNow(1) / NSTerminateLater(2)
    if delegate != 0 {
        let reply =
            dispatch_u64_selector_named_to_target(delegate, "applicationShouldTerminate:", obj);
        // reply == u64::MAX means the delegate does not respond — treat as NSTerminateNow.
        if reply != u64::MAX && reply == 0 {
            log::debug!("NSApplication terminate: cancelled by delegate");
            return;
        }
    }

    if delegate != 0 {
        send_delegate_message(delegate, "applicationWillTerminate:", 0);
    }

    unsafe {
        *((obj as usize + IS_RUNNING_OFF) as *mut u64) = 0;
    }

    // Ask GUI bridge to stop if active
    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        bridge.request_stop();
    }

    log::debug!("NSApplication terminate: stopped");
}

/// `-[NSApplication activateIgnoringOtherApps:]` — stub, no-op.
pub extern "C" fn shim_nsapplication_activate(_obj: u64, _cmd: u64, _flag: u8) {
    log::debug!("NSApplication activateIgnoringOtherApps: (stub)");
}

/// `-[NSApplication finishLaunching]` — deliver launch notifications once.
pub extern "C" fn shim_nsapplication_finish_launching(obj: u64, _cmd: u64) {
    if obj == 0 || !mark_launch_notifications_pending(obj) {
        return;
    }

    log::debug!("NSApplication finishLaunching");

    send_launch_notification(
        obj,
        "NSApplicationWillFinishLaunchingNotification",
        "applicationWillFinishLaunching:",
    );
    send_launch_notification(
        obj,
        "NSApplicationDidFinishLaunchingNotification",
        "applicationDidFinishLaunching:",
    );
}

/// `-[NSApplication setActivationPolicy:]` — stub, returns YES.
pub extern "C" fn shim_nsapplication_set_activation_policy(
    _obj: u64,
    _cmd: u64,
    _policy: i64,
) -> u8 {
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
        current = rt
            .get_class_by_addr(addr)
            .and_then(|class| class.superclass_addr);
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

/// Dispatch a selector with **two** arguments through the ObjC runtime.
pub(crate) fn dispatch_selector_named_2arg(
    target: u64,
    selector: &str,
    arg1: u64,
    arg2: u64,
) -> bool {
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

    let func: extern "C" fn(u64, u64, u64, u64) = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, arg1, arg2);
    true
}

/// Dispatch a BOOL-returning selector with one argument through the ObjC runtime.
/// Returns `false` when the runtime or method is unavailable.
pub(crate) fn dispatch_bool_selector_named_to_target(
    target: u64,
    selector: &str,
    arg: u64,
) -> bool {
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

fn mark_launch_notifications_pending(app: u64) -> bool {
    if app == 0 {
        return false;
    }

    unsafe {
        let slot = (app as usize + FINISHED_LAUNCHING_OFF) as *mut u64;
        if *slot != 0 {
            return false;
        }
        *slot = 1;
    }

    true
}

fn create_launch_notification(app: u64, notification_name: &str) -> u64 {
    let Ok(name_cstr) = std::ffi::CString::new(notification_name) else {
        return 0;
    };
    let name = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
    crate::nsnotificationcenter::make_nsnotification(name, app, 0)
}

fn send_launch_notification(app: u64, notification_name: &str, delegate_selector: &str) {
    let notification = create_launch_notification(app, notification_name);
    if notification == 0 {
        return;
    }

    let center = crate::nsnotificationcenter::shim_nsnotificationcenter_default_center(0, 0);
    if center != 0 {
        crate::nsnotificationcenter::shim_nsnotificationcenter_post_notification(
            center,
            0,
            notification,
        );
    }

    let delegate = unsafe { *((app as usize + DELEGATE_OFF) as *const u64) };
    if delegate != 0 {
        send_delegate_message(delegate, delegate_selector, notification);
    }
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

/// Dispatch a u64-returning selector with one argument through the ObjC runtime.
/// Returns `u64::MAX` when the runtime or method is unavailable.
pub(crate) fn dispatch_u64_selector_named_to_target(target: u64, selector: &str, arg: u64) -> u64 {
    if target == 0 || selector.is_empty() {
        return u64::MAX;
    }

    let isa = unsafe { *(target as *const u64) };

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return u64::MAX;
    };

    let Some(imp) = rt.msg_lookup(isa, selector) else {
        return u64::MAX;
    };

    let sel_ptr = rt.selector_ptr(selector).unwrap_or(0) as u64;
    drop(guard);

    let func: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, arg)
}

/// `-[NSApplication nextEventMatchingMask:untilDate:inMode:dequeue:]`
///
/// Returns the next event in the queue whose type matches `mask`, or nil if
/// none is available. When `dequeue` is YES (non-zero), the event is removed
/// from the queue.
pub extern "C" fn shim_nsapplication_next_event(
    _obj: u64,
    _cmd: u64,
    mask: u64,
    _until_date: u64,
    _mode: u64,
    dequeue: u8,
) -> u64 {
    dequeue_event(mask, dequeue != 0)
}

/// `-[NSApplication postEvent:atStart:]`
///
/// Inserts an event into the application event queue. If `at_start` is YES,
/// the event is placed at the front of the queue; otherwise at the end.
pub extern "C" fn shim_nsapplication_post_event(_obj: u64, _cmd: u64, event: u64, at_start: u8) {
    if event == 0 {
        return;
    }
    let mut queue = event_queue().lock().unwrap_or_else(|e| e.into_inner());
    if at_start != 0 {
        queue.push_front(event);
    } else {
        queue.push_back(event);
    }
}

/// `-[NSApplication discardEventsMatchingMask:beforeEvent:]`
pub extern "C" fn shim_nsapplication_discard_events(
    _obj: u64,
    _cmd: u64,
    mask: u64,
    before_event: u64,
) {
    let match_any = mask == u64::MAX || mask == 0;
    let mut queue = event_queue().lock().unwrap_or_else(|e| e.into_inner());
    queue.retain(|&event| {
        // If before_event is non-zero, only discard events that came before it.
        if before_event != 0 && event == before_event {
            return true; // keep this event and everything after it
        }
        if match_any || event_matches_mask(event, mask) {
            return false; // discard
        }
        true
    });
}

/// `-[NSApplication updateWindows]`
///
/// Sends `display` to every visible window that has been marked as needing
/// display. This is the standard AppKit mechanism for flushing dirty views.
pub extern "C" fn shim_nsapplication_update_windows(_obj: u64, _cmd: u64) {
    for &window in &snapshot_windows() {
        if crate::nswindow::is_window_visible(window) {
            let content_view = crate::nswindow::window_content_view(window);
            if content_view != 0 && crate::nsview::view_needs_display(content_view) {
                crate::nsview::shim_nsview_display(content_view, 0);
            }
        }
    }
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

fn make_nsstring(value: &str) -> u64 {
    let Ok(cstr) = std::ffi::CString::new(value) else {
        return 0;
    };
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

fn main_bundle_info_value(key: &str) -> u64 {
    let bundle = crate::nsbundle::get_or_create_main_bundle();
    if bundle == 0 {
        return 0;
    }
    let key = make_nsstring(key);
    if key == 0 {
        return 0;
    }
    crate::nsbundle::shim_nsbundle_object_for_info_key(bundle, 0, key)
}

fn main_bundle_info_string(key: &str) -> Option<(u64, String)> {
    let value = main_bundle_info_value(key);
    if value == 0 {
        return None;
    }
    crate::cfstring::extract_cfstring_cstr(value).map(|s| (value, s.to_string()))
}

fn runtime_class_is_subclass_of(
    rt: &objc_runtime::ObjcRuntime,
    class_addr: u64,
    superclass_addr: u64,
) -> bool {
    let mut current = Some(class_addr);
    let mut depth = 0;
    while let Some(addr) = current {
        if depth > 64 {
            break;
        }
        if addr == superclass_addr {
            return true;
        }
        current = rt
            .get_class_by_addr(addr)
            .and_then(|class| class.superclass_addr);
        depth += 1;
    }
    false
}

fn configure_principal_application_class_from_info_plist() {
    let Some((_name_obj, class_name)) = main_bundle_info_string("NSPrincipalClass") else {
        return;
    };
    if class_name.is_empty() {
        return;
    }

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        log::debug!("NSApplicationMain: runtime unavailable for NSPrincipalClass={class_name}");
        return;
    };
    let Some(class_info) = rt.get_class(&class_name) else {
        log::debug!(
            "NSApplicationMain: NSPrincipalClass '{}' not registered",
            class_name
        );
        return;
    };
    let class_addr = class_info.mapped_addr;
    if !runtime_class_is_subclass_of(rt, class_addr, crate::nsobj::nsapplication_class_addr()) {
        log::warn!(
            "NSApplicationMain: NSPrincipalClass '{}' is not an NSApplication subclass",
            class_name
        );
        return;
    }
    drop(guard);

    set_principal_application_class(class_addr);
    log::info!("NSApplicationMain: using principal class {}", class_name);
}

fn normalized_nib_name_for_loading(nib_obj: u64, nib_name: &str) -> u64 {
    nib_name
        .strip_suffix(".nib")
        .map(make_nsstring)
        .filter(|&name| name != 0)
        .unwrap_or(nib_obj)
}

fn normalized_storyboard_name_for_loading(storyboard_obj: u64, storyboard_name: &str) -> u64 {
    storyboard_name
        .strip_suffix(".storyboardc")
        .or_else(|| storyboard_name.strip_suffix(".storyboard"))
        .map(make_nsstring)
        .filter(|&name| name != 0)
        .unwrap_or(storyboard_obj)
}

fn load_main_storyboard_from_info_plist(app: u64) -> bool {
    if app == 0 {
        return false;
    }

    let Some((storyboard_obj, storyboard_name)) = main_bundle_info_string("NSMainStoryboardFile")
    else {
        return false;
    };

    let bundle = crate::nsbundle::get_or_create_main_bundle();
    if bundle == 0 {
        return false;
    }

    let storyboard_name_for_loading =
        normalized_storyboard_name_for_loading(storyboard_obj, &storyboard_name);
    let storyboard = crate::nsstoryboard::shim_nsstoryboard_with_name_bundle(
        crate::nsobj::nsstoryboard_class_addr(),
        0,
        storyboard_name_for_loading,
        bundle,
    );
    if storyboard == 0 {
        log::warn!(
            "NSApplicationMain: failed to create main storyboard '{}'",
            storyboard_name
        );
        return false;
    }

    let controller =
        crate::nsstoryboard::shim_nsstoryboard_instantiate_initial_controller(storyboard, 0);
    if controller == 0 {
        log::warn!(
            "NSApplicationMain: failed to instantiate main storyboard '{}'",
            storyboard_name
        );
        return false;
    }

    if crate::nsstoryboard::present_storyboard_controller(controller, app) {
        log::info!(
            "NSApplicationMain: loaded main storyboard '{}'",
            storyboard_name
        );
    } else {
        log::info!(
            "NSApplicationMain: instantiated main storyboard '{}' controller {:#x}",
            storyboard_name,
            controller
        );
    }
    true
}

fn load_main_nib_from_info_plist(app: u64) -> bool {
    if app == 0 {
        return false;
    }

    let Some((nib_obj, nib_name)) = main_bundle_info_string("NSMainNibFile") else {
        return load_main_storyboard_from_info_plist(app);
    };

    let bundle = crate::nsbundle::get_or_create_main_bundle();
    if bundle == 0 {
        return false;
    }

    let mut top_level = 0_u64;
    let nib_for_loading = normalized_nib_name_for_loading(nib_obj, &nib_name);
    let ok = crate::nsbundle::shim_nsbundle_load_nib_named(
        bundle,
        0,
        nib_for_loading,
        app,
        (&mut top_level as *mut u64) as u64,
    );
    if ok != 0 {
        log::info!("NSApplicationMain: loaded main nib '{}'", nib_name);
        true
    } else {
        log::warn!("NSApplicationMain: failed to load main nib '{}'", nib_name);
        false
    }
}

// ── NSApplicationMain C entry point ────────────────────────────

/// `NSApplicationMain(argc, argv)` — the standard C entry point for Cocoa apps.
///
/// Creates the shared application, honors `NSPrincipalClass` when it names a
/// registered NSApplication subclass, loads `NSMainNibFile`/`NSMainStoryboardFile`,
/// and calls `[NSApp run]`.
pub extern "C" fn shim_nsapplication_main(_argc: i32, _argv: u64) -> i32 {
    log::info!("NSApplicationMain: entering");
    configure_principal_application_class_from_info_plist();
    let app = get_or_create_shared_app();
    load_main_nib_from_info_plist(app);

    // Enter the run loop — this blocks until termination.
    shim_nsapplication_run(app, 0);

    // NSApplicationMain never returns in a real app, but we return 0
    // for graceful exit after the run loop terminates.
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;
    use plist::{Dictionary, Value};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SIZE: usize = APP_SIZE;

    static APP_TEST_LOCK: Mutex<()> = Mutex::new(());
    static ACTION_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_ACTION_TARGET: AtomicU64 = AtomicU64::new(0);
    static LAST_ACTION_SENDER: AtomicU64 = AtomicU64::new(0);
    static EVENT_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_TARGET: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_ARG: AtomicU64 = AtomicU64::new(0);
    static CURRENT_EVENT_DURING_EVENT_CALL: AtomicU64 = AtomicU64::new(0);
    static LAUNCH_WILL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAUNCH_DID_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_WILL_NOTIFICATION: AtomicU64 = AtomicU64::new(0);
    static LAST_DID_NOTIFICATION: AtomicU64 = AtomicU64::new(0);

    const ACTION_PROBE_CLASS_ADDR: u64 = 0x0000_0001_0050_1000;
    const LAUNCH_PROBE_CLASS_ADDR: u64 = 0x0000_0001_0050_2000;

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
        CURRENT_EVENT_DURING_EVENT_CALL
            .store(shim_nsapplication_current_event(app, 0), Ordering::SeqCst);
    }

    extern "C" fn launch_probe_will_imp(_obj: u64, _cmd: u64, notification: u64) {
        LAUNCH_WILL_COUNT.fetch_add(1, Ordering::SeqCst);
        LAST_WILL_NOTIFICATION.store(notification, Ordering::SeqCst);
    }

    extern "C" fn launch_probe_did_imp(_obj: u64, _cmd: u64, notification: u64) {
        LAUNCH_DID_COUNT.fetch_add(1, Ordering::SeqCst);
        LAST_DID_NOTIFICATION.store(notification, Ordering::SeqCst);
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
        LAUNCH_WILL_COUNT.store(0, Ordering::SeqCst);
        LAUNCH_DID_COUNT.store(0, Ordering::SeqCst);
        LAST_WILL_NOTIFICATION.store(0, Ordering::SeqCst);
        LAST_DID_NOTIFICATION.store(0, Ordering::SeqCst);
    }

    fn install_action_probe_class(rt: &mut objc_runtime::ObjcRuntime) {
        let mut info = objc_runtime::ClassInfo {
            name: "ActionProbe".to_string(),
            mapped_addr: ACTION_PROBE_CLASS_ADDR,
            methods: vec![
                objc_runtime::MethodEntry {
                    selector: "performAction:".into(),
                    imp: action_probe_imp as *const () as u64,
                    type_encoding: "v@:@".into(),
                },
                objc_runtime::MethodEntry {
                    selector: "keyDown:".into(),
                    imp: event_probe_imp as *const () as u64,
                    type_encoding: "v@:@".into(),
                },
            ],
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

    fn install_launch_probe_class(rt: &mut objc_runtime::ObjcRuntime) {
        let mut info = objc_runtime::ClassInfo {
            name: "LaunchProbe".to_string(),
            mapped_addr: LAUNCH_PROBE_CLASS_ADDR,
            methods: vec![
                objc_runtime::MethodEntry {
                    selector: "applicationWillFinishLaunching:".into(),
                    imp: launch_probe_will_imp as *const () as u64,
                    type_encoding: "v@:@".into(),
                },
                objc_runtime::MethodEntry {
                    selector: "applicationDidFinishLaunching:".into(),
                    imp: launch_probe_did_imp as *const () as u64,
                    type_encoding: "v@:@".into(),
                },
            ],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 16,
            is_root: false,
            is_meta: false,
            superclass_addr: Some(crate::nsobj::nsobject_class_addr()),
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    fn with_action_runtime<R>(f: impl FnOnce() -> R) -> R {
        let saved_runtime = {
            let mut guard = objc_runtime::OBJC_RUNTIME
                .write()
                .unwrap_or_else(|e| e.into_inner());
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

        let mut guard = objc_runtime::OBJC_RUNTIME
            .write()
            .unwrap_or_else(|e| e.into_inner());
        *guard = saved_runtime;
        result
    }

    fn with_launch_runtime<R>(f: impl FnOnce() -> R) -> R {
        let saved_runtime = {
            let mut guard = objc_runtime::OBJC_RUNTIME
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let saved = std::mem::take(&mut *guard);

            let mut rt = objc_runtime::ObjcRuntime::new();
            crate::FoundationShims::register_nsobject_class(&mut rt);
            crate::FoundationShims::register_nsstring_class(&mut rt);
            crate::FoundationShims::register_nsarray_class(&mut rt);
            crate::FoundationShims::register_nsmutablearray_class(&mut rt);
            crate::FoundationShims::register_nsnumber_class(&mut rt);
            crate::FoundationShims::register_nsbundle_class(&mut rt);
            crate::FoundationShims::register_nsresponder_class(&mut rt);
            crate::FoundationShims::register_nsapplication_class(&mut rt);
            crate::FoundationShims::register_nsnotification_class(&mut rt);
            crate::FoundationShims::register_nsnotificationcenter_class(&mut rt);
            crate::FoundationShims::register_nswindow_class(&mut rt);
            crate::FoundationShims::register_nsview_class(&mut rt);
            crate::FoundationShims::register_nscontrol_class(&mut rt);
            crate::FoundationShims::register_nsbutton_class(&mut rt);
            crate::FoundationShims::register_nsimage_class(&mut rt);
            crate::FoundationShims::register_nsimageview_class(&mut rt);
            crate::FoundationShims::register_nsviewcontroller_class(&mut rt);
            crate::FoundationShims::register_nswindowcontroller_class(&mut rt);
            crate::FoundationShims::register_nsstoryboard_class(&mut rt);
            install_launch_probe_class(&mut rt);

            *guard = Some(rt);
            saved
        };

        let result = f();

        let mut guard = objc_runtime::OBJC_RUNTIME
            .write()
            .unwrap_or_else(|e| e.into_inner());
        *guard = saved_runtime;
        result
    }

    fn intern_selector(name: &str) -> u64 {
        let mut guard = objc_runtime::OBJC_RUNTIME
            .write()
            .unwrap_or_else(|e| e.into_inner());
        guard
            .as_mut()
            .expect("test ObjC runtime should be installed")
            .intern_selector(name) as u64
    }

    fn notification_name(notification: u64) -> String {
        let name = crate::nsnotificationcenter::shim_nsnotification_name(notification, 0);
        crate::cfstring::extract_cfstring_cstr(name)
            .unwrap_or("")
            .to_string()
    }

    fn unique_temp_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "burdettie-nsapp-main-storyboard-{name}-{}-{stamp}",
            std::process::id()
        ))
    }

    const ONE_PIXEL_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 1, 115, 82, 71, 66, 0, 174, 206, 28, 233, 0, 0, 0, 4,
        103, 65, 77, 65, 0, 0, 177, 143, 11, 252, 97, 5, 0, 0, 0, 9, 112, 72, 89, 115, 0, 0, 14,
        195, 0, 0, 14, 195, 1, 199, 111, 168, 100, 0, 0, 0, 13, 73, 68, 65, 84, 24, 87, 99, 248,
        207, 192, 240, 31, 0, 5, 0, 1, 255, 166, 92, 155, 93, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
        96, 130,
    ];

    fn uid(id: u64) -> Value {
        Value::Uid(plist::Uid::new(id))
    }

    fn class_descriptor(name: &str, classes: &[&str]) -> Value {
        let mut dict = Dictionary::new();
        dict.insert("$classname".to_string(), Value::String(name.to_string()));
        dict.insert(
            "$classes".to_string(),
            Value::Array(
                classes
                    .iter()
                    .map(|class| Value::String((*class).to_string()))
                    .collect(),
            ),
        );
        Value::Dictionary(dict)
    }

    fn keyed_object(class_uid: u64, entries: Vec<(&str, Value)>) -> Value {
        let mut dict = Dictionary::new();
        dict.insert("$class".to_string(), uid(class_uid));
        for (key, value) in entries {
            dict.insert(key.to_string(), value);
        }
        Value::Dictionary(dict)
    }

    fn plist_xml_bytes(dict: Dictionary) -> Vec<u8> {
        let mut bytes = Vec::new();
        plist::to_writer_xml(&mut bytes, &Value::Dictionary(dict)).unwrap();
        bytes
    }

    fn build_apple_archive(objects: Vec<Value>, root_uid: u64) -> Vec<u8> {
        let mut top = Dictionary::new();
        top.insert("root".to_string(), uid(root_uid));

        let mut root = Dictionary::new();
        root.insert(
            "$archiver".to_string(),
            Value::String("NSKeyedArchiver".to_string()),
        );
        root.insert("$version".to_string(), Value::Integer(100000.into()));
        root.insert("$top".to_string(), Value::Dictionary(top));
        root.insert("$objects".to_string(), Value::Array(objects));

        let mut bytes = Vec::new();
        plist::to_writer_binary(&mut bytes, &Value::Dictionary(root)).unwrap();
        bytes
    }

    fn storyboard_info_bytes() -> Vec<u8> {
        let mut scene_map = Dictionary::new();
        scene_map.insert(
            "main-window".to_string(),
            Value::String("MainWindowController".to_string()),
        );

        let mut info = Dictionary::new();
        info.insert(
            "NSStoryboardInitialControllerIdentifier".to_string(),
            Value::String("main-window".to_string()),
        );
        info.insert(
            "NSStoryboardSceneIdentifiersToNibNames".to_string(),
            Value::Dictionary(scene_map),
        );
        plist_xml_bytes(info)
    }

    fn scene_nib_bytes() -> Vec<u8> {
        build_apple_archive(
            vec![
                Value::String("$null".to_string()),
                keyed_object(10, vec![("NS.objects", Value::Array(vec![uid(2)]))]),
                keyed_object(
                    11,
                    vec![
                        ("NSWindow", uid(3)),
                        (
                            "NSWindowNibName",
                            Value::String("MainWindowController".to_string()),
                        ),
                        ("NSShouldCascadeWindows", Value::Boolean(false)),
                    ],
                ),
                keyed_object(
                    12,
                    vec![
                        (
                            "NSWindowFrame",
                            Value::String("{{44, 55}, {640, 480}}".to_string()),
                        ),
                        (
                            "NSTitle",
                            Value::String("App Bundle Storyboard".to_string()),
                        ),
                        ("NSWindowStyleMask", Value::Integer(15.into())),
                        ("NSContentView", uid(6)),
                    ],
                ),
                Value::String("{{44, 55}, {640, 480}}".to_string()),
                Value::String("App Bundle Storyboard".to_string()),
                keyed_object(
                    13,
                    vec![
                        ("NSFrame", Value::String("{{0, 0}, {640, 480}}".to_string())),
                        ("NSSubviews", uid(14)),
                    ],
                ),
                Value::String("MainWindowController".to_string()),
                Value::Boolean(false),
                Value::String("{{0, 0}, {640, 480}}".to_string()),
                class_descriptor("NSArray", &["NSArray", "NSObject"]),
                class_descriptor(
                    "NSWindowController",
                    &["NSWindowController", "NSResponder", "NSObject"],
                ),
                class_descriptor("NSWindow", &["NSWindow", "NSResponder", "NSObject"]),
                class_descriptor("NSView", &["NSView", "NSResponder", "NSObject"]),
                keyed_object(
                    10,
                    vec![("NS.objects", Value::Array(vec![uid(15), uid(20)]))],
                ),
                keyed_object(
                    16,
                    vec![
                        (
                            "NSFrame",
                            Value::String("{{12, 18}, {120, 40}}".to_string()),
                        ),
                        ("NSImage", uid(18)),
                    ],
                ),
                class_descriptor(
                    "NSImageView",
                    &[
                        "NSImageView",
                        "NSControl",
                        "NSView",
                        "NSResponder",
                        "NSObject",
                    ],
                ),
                Value::String("{{12, 18}, {120, 40}}".to_string()),
                keyed_object(
                    19,
                    vec![("NSName", Value::String("ToolbarBadgeTemplate".to_string()))],
                ),
                class_descriptor("NSImage", &["NSImage", "NSObject"]),
                keyed_object(
                    21,
                    vec![
                        (
                            "NSFrame",
                            Value::String("{{24, 76}, {180, 32}}".to_string()),
                        ),
                        ("NSTitle", uid(23)),
                        ("NSImage", uid(24)),
                        ("NSEnabled", Value::Boolean(true)),
                        ("NSBordered", Value::Boolean(true)),
                    ],
                ),
                class_descriptor(
                    "NSButton",
                    &["NSButton", "NSControl", "NSView", "NSResponder", "NSObject"],
                ),
                Value::String("{{24, 76}, {180, 32}}".to_string()),
                Value::String("Continue".to_string()),
                keyed_object(
                    19,
                    vec![
                        (
                            "NSName",
                            Value::String("NSImageNameGoRightTemplate".to_string()),
                        ),
                        ("NSTemplate", Value::Boolean(true)),
                    ],
                ),
            ],
            1,
        )
    }

    fn write_storyboard_asset_catalog(resources: &Path) {
        let image_set = resources
            .join("Assets.xcassets")
            .join("ToolbarBadgeTemplate.imageset");
        std::fs::create_dir_all(&image_set).unwrap();
        std::fs::write(image_set.join("ToolbarBadgeTemplate.png"), ONE_PIXEL_PNG).unwrap();
        std::fs::write(
            image_set.join("Contents.json"),
            r#"{
  "images": [
    { "idiom": "universal", "filename": "ToolbarBadgeTemplate.png", "scale": "1x" }
  ],
  "properties": { "template-rendering-intent": "template" },
  "info": { "version": 1, "author": "xcode" }
}"#,
        )
        .unwrap();
    }

    fn write_app_bundle_storyboard_fixture(app: &Path) {
        let mut app_info = Dictionary::new();
        app_info.insert(
            "CFBundleIdentifier".to_string(),
            Value::String("dev.burdettie.tests.main-storyboard".to_string()),
        );
        app_info.insert(
            "NSPrincipalClass".to_string(),
            Value::String("NSApplication".to_string()),
        );
        app_info.insert(
            "NSMainStoryboardFile".to_string(),
            Value::String("Main".to_string()),
        );

        let contents = app.join("Contents");
        let resources = contents.join("Resources");
        let storyboard_dir = resources.join("Base.lproj").join("Main.storyboardc");
        let nib_dir = storyboard_dir.join("MainWindowController.nib");

        std::fs::create_dir_all(&nib_dir).unwrap();
        write_storyboard_asset_catalog(&resources);
        std::fs::write(contents.join("Info.plist"), plist_xml_bytes(app_info)).unwrap();
        std::fs::write(storyboard_dir.join("Info.plist"), storyboard_info_bytes()).unwrap();
        std::fs::write(nib_dir.join("keyedobjects.nib"), scene_nib_bytes()).unwrap();
    }

    fn make_temp_storyboard_app(name: &str) -> (PathBuf, PathBuf) {
        let root = unique_temp_root(name);
        let app = root.join("Fixture.app");
        write_app_bundle_storyboard_fixture(&app);
        (root, app)
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
    fn windows_menu_roundtrip() {
        let app = make_app();
        shim_nsapplication_set_windows_menu(app, 0, 0x1234);
        assert_eq!(shim_nsapplication_windows_menu(app, 0), 0x1234);
        free_obj(app);
    }

    #[test]
    fn help_menu_roundtrip() {
        let app = make_app();
        shim_nsapplication_set_help_menu(app, 0, 0x5678);
        assert_eq!(shim_nsapplication_help_menu(app, 0), 0x5678);
        free_obj(app);
    }

    #[test]
    fn finish_launching_sends_launch_callbacks_once_with_notifications() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_launch_runtime(|| {
            reset_action_probe();

            let app = shim_nsapplication_shared_application(0, 0);
            let delegate = alloc_obj(16, LAUNCH_PROBE_CLASS_ADDR);
            shim_nsapplication_set_delegate(app, 0, delegate);

            shim_nsapplication_finish_launching(app, 0);
            shim_nsapplication_finish_launching(app, 0);

            assert_eq!(LAUNCH_WILL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAUNCH_DID_COUNT.load(Ordering::SeqCst), 1);

            let will_notification = LAST_WILL_NOTIFICATION.load(Ordering::SeqCst);
            let did_notification = LAST_DID_NOTIFICATION.load(Ordering::SeqCst);
            assert_ne!(will_notification, 0);
            assert_ne!(did_notification, 0);
            assert_eq!(
                notification_name(will_notification),
                "NSApplicationWillFinishLaunchingNotification"
            );
            assert_eq!(
                notification_name(did_notification),
                "NSApplicationDidFinishLaunchingNotification"
            );
            assert_eq!(
                crate::nsnotificationcenter::shim_nsnotification_object(will_notification, 0),
                app
            );
            assert_eq!(
                crate::nsnotificationcenter::shim_nsnotification_object(did_notification, 0),
                app
            );

            free_obj(delegate);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn run_triggers_finish_launching_when_needed() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_launch_runtime(|| {
            reset_action_probe();

            let app = shim_nsapplication_shared_application(0, 0);
            let delegate = alloc_obj(16, LAUNCH_PROBE_CLASS_ADDR);
            shim_nsapplication_set_delegate(app, 0, delegate);

            shim_nsapplication_run(app, 0);

            assert_eq!(LAUNCH_WILL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAUNCH_DID_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(shim_nsapplication_is_running(app, 0), 0);

            free_obj(delegate);
        });

        reset_runtime_state_for_reuse();
    }

    #[test]
    fn application_main_loads_bundle_main_storyboard_and_presents_window() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_launch_runtime(|| {
            let (root, app_bundle) = make_temp_storyboard_app("application-main");
            let bundle = crate::nsbundle::set_main_bundle_path_for_test(&app_bundle);
            assert_ne!(bundle, 0);

            assert_eq!(shim_nsapplication_main(0, 0), 0);

            let app = shim_nsapplication_shared_application(0, 0);
            let window = shim_nsapplication_key_window(app, 0);
            assert_ne!(window, 0);
            assert_eq!(shim_nsapplication_main_window(app, 0), window);
            assert_eq!(crate::nswindow::shim_nswindow_is_visible(window, 0), 1);
            assert_eq!(shim_nsapplication_is_running(app, 0), 0);

            let title = crate::nswindow::shim_nswindow_title(window, 0);
            assert_string_eq(title, "App Bundle Storyboard");

            let frame = crate::nswindow::shim_nswindow_frame(window, 0);
            assert_eq!(frame.origin.x, 44.0);
            assert_eq!(frame.origin.y, 55.0);
            assert_eq!(frame.size.width, 640.0);
            assert_eq!(frame.size.height, 480.0);

            let content_view = crate::nswindow::shim_nswindow_content_view(window, 0);
            assert_ne!(content_view, 0);
            let subviews = crate::nsview::shim_nsview_subviews(content_view, 0);
            assert_eq!(crate::nsarray::shim_nsarray_count(subviews, 0), 2);
            let image_child = crate::nsarray::shim_nsarray_object_at_index(subviews, 0, 0);
            assert_ne!(image_child, 0);
            assert_eq!(
                unsafe { *(image_child as *const u64) },
                crate::nsobj::nsimageview_class_addr()
            );
            assert_eq!(
                crate::nsview::shim_nsview_superview(image_child, 0),
                content_view
            );
            let child_frame = crate::nsview::shim_nsview_frame(image_child, 0);
            assert_eq!(child_frame.origin.x, 12.0);
            assert_eq!(child_frame.origin.y, 18.0);
            assert_eq!(child_frame.size.width, 120.0);
            assert_eq!(child_frame.size.height, 40.0);

            let image = crate::nsimageview::shim_nsimageview_image(image_child, 0);
            assert_ne!(image, 0);
            assert_ne!(crate::nsimage::image_to_cg_handle(image), 0);
            assert_eq!(crate::nsimage::shim_nsimage_is_template(image, 0), 1);
            let image_path = crate::nsimage::nsimage_file_path(image)
                .expect("storyboard image should resolve to a source asset catalog file");
            assert!(
                image_path.replace('\\', "/").ends_with(
                    "Assets.xcassets/ToolbarBadgeTemplate.imageset/ToolbarBadgeTemplate.png"
                ),
                "unexpected storyboard image path: {image_path}"
            );
            let image_name = crate::nsimage::shim_nsimage_name(image, 0);
            assert_string_eq(image_name, "ToolbarBadgeTemplate");

            let button = crate::nsarray::shim_nsarray_object_at_index(subviews, 0, 1);
            assert_ne!(button, 0);
            assert_eq!(
                unsafe { *(button as *const u64) },
                crate::nsobj::nsbutton_class_addr()
            );
            assert_eq!(
                crate::nsview::shim_nsview_superview(button, 0),
                content_view
            );
            let button_frame = crate::nsview::shim_nsview_frame(button, 0);
            assert_eq!(button_frame.origin.x, 24.0);
            assert_eq!(button_frame.origin.y, 76.0);
            assert_eq!(button_frame.size.width, 180.0);
            assert_eq!(button_frame.size.height, 32.0);
            assert_string_eq(crate::nsbutton::shim_nsbutton_title(button, 0), "Continue");
            assert_eq!(crate::nsbutton::shim_nsbutton_is_enabled(button, 0), 1);
            assert_eq!(crate::nsbutton::shim_nsbutton_is_bordered(button, 0), 1);

            let button_image = crate::nsbutton::shim_nsbutton_image(button, 0);
            assert_ne!(button_image, 0);
            assert_ne!(crate::nsimage::image_to_cg_handle(button_image), 0);
            assert_eq!(crate::nsimage::shim_nsimage_is_template(button_image, 0), 1);
            assert_string_eq(
                crate::nsimage::shim_nsimage_name(button_image, 0),
                "NSImageNameGoRightTemplate",
            );

            crate::nswindow::display_window_content(window);
            let ctx = crate::nswindow::get_or_create_window_cg_context(window);
            let rendered_pixels = cg_shims::bridge::with_context(ctx, |c| {
                let white = graphics_core::Color::WHITE.to_argb32();
                c.surface
                    .pixels()
                    .iter()
                    .filter(|&&pixel| pixel != white)
                    .count()
            })
            .unwrap_or(0);
            assert!(
                rendered_pixels > 0,
                "storyboard image view should render non-white pixels into the window surface"
            );
            let button_pixels = cg_shims::bridge::with_context(ctx, |c| {
                let white = graphics_core::Color::WHITE.to_argb32();
                let width = c.surface.width() as usize;
                let height = c.surface.height() as usize;
                let pixels = c.surface.pixels();
                let x0 = 24_usize.min(width);
                let x1 = 204_usize.min(width);
                let y0 = 76_usize.min(height);
                let y1 = 108_usize.min(height);
                (y0..y1)
                    .flat_map(|y| (x0..x1).map(move |x| y * width + x))
                    .filter(|&idx| pixels.get(idx).copied().unwrap_or(white) != white)
                    .count()
            })
            .unwrap_or(0);
            assert!(
                button_pixels > 0,
                "storyboard button should render non-white pixels in its own frame"
            );

            crate::nsbundle::clear_main_bundle_path_for_test();
            std::fs::remove_dir_all(root).unwrap();
        });

        reset_runtime_state_for_reuse();
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
            assert_eq!(
                crate::nswindow::shim_nswindow_make_first_responder(window, 0, first),
                1
            );

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

            assert_eq!(
                shim_nsapplication_target_for_action(app, 0, action),
                delegate
            );

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
            assert_eq!(
                crate::nswindow::shim_nswindow_make_first_responder(window, 0, target),
                1
            );

            shim_nsapplication_send_event(app, 0, event);

            assert_eq!(EVENT_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_EVENT_TARGET.load(Ordering::SeqCst), target);
            assert_eq!(LAST_EVENT_ARG.load(Ordering::SeqCst), event);
            assert_eq!(
                CURRENT_EVENT_DURING_EVENT_CALL.load(Ordering::SeqCst),
                event
            );
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
            assert_eq!(
                crate::nswindow::shim_nswindow_make_first_responder(first_window, 0, first_target),
                1
            );
            crate::nswindow::shim_nswindow_make_key_and_order_front(second_window, 0, 0);
            assert_eq!(
                crate::nswindow::shim_nswindow_make_first_responder(
                    second_window,
                    0,
                    second_target
                ),
                1
            );

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
    fn run_headless_single_pass_returns_not_running() {
        let app = make_app();
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        shim_nsapplication_run(app, 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
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
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0),
            window
        );

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
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 0),
            second
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 1),
            first
        );

        crate::nswindow::shim_nswindow_order_out(second, 0, 0);
        let visible_only = shim_nsapplication_ordered_windows(app, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(visible_only, 0), 1);
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(visible_only, 0, 0),
            first
        );

        crate::nswindow::shim_nswindow_order_front(second, 0, 0);
        crate::nswindow::shim_nswindow_order_front(first, 0, 0);
        let reordered = shim_nsapplication_ordered_windows(app, 0);
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(reordered, 0, 0),
            first
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(reordered, 0, 1),
            second
        );

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

            assert_eq!(
                shim_nsapplication_send_action_to_from(app, 0, action, target, sender),
                1
            );
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
            assert_eq!(
                crate::nswindow::shim_nswindow_make_first_responder(window, 0, first),
                1
            );

            assert_eq!(
                shim_nsapplication_send_action_to_from(app, 0, action, 0, sender),
                1
            );
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

            assert_eq!(
                shim_nsapplication_send_action_to_from(app, 0, action, 0, sender),
                1
            );
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

            assert_eq!(
                shim_nsapplication_send_action_to_from(app, 0, action, 0, 0xDEAD),
                0
            );
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
        assert_eq!(
            crate::nsarray::shim_nsarray_count(shim_nsapplication_windows(app, 0), 0),
            0
        );
        free_obj(window);
    }

    #[test]
    fn run_then_terminate_cycle() {
        let app = make_app();
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
        shim_nsapplication_run(app, 0);
        assert_eq!(shim_nsapplication_is_running(app, 0), 0);
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
        assert_eq!(
            shim_nsapplication_next_event(app, 0, 0xFFFF_FFFF, 0, 0, 1),
            0
        );
        free_obj(app);
    }

    #[test]
    fn next_event_nil_does_not_crash() {
        assert_eq!(shim_nsapplication_next_event(0, 0, 0, 0, 0, 0), 0);
    }

    // ---- shim_nsapplication_main ----

    #[test]
    fn nsapplication_main_returns_zero() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();
        // With no GUI bridge, run exits immediately and main returns 0.
        let result = shim_nsapplication_main(0, 0);
        assert_eq!(result, 0);
        reset_runtime_state_for_reuse();
    }

    // ---- dispatch_u64_selector_named_to_target ----

    #[test]
    fn dispatch_u64_selector_nil_target_returns_max() {
        assert_eq!(
            dispatch_u64_selector_named_to_target(0, "someMethod:", 0),
            u64::MAX
        );
    }

    #[test]
    fn dispatch_u64_selector_empty_selector_returns_max() {
        let target = alloc_obj(16, crate::nsobj::nsresponder_class_addr());
        assert_eq!(
            dispatch_u64_selector_named_to_target(target, "", 0),
            u64::MAX
        );
        free_obj(target);
    }

    #[test]
    fn dispatch_u64_selector_non_responding_target_returns_max() {
        let _guard = APP_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_reuse();

        with_action_runtime(|| {
            // A plain NSResponder doesn't respond to "applicationShouldTerminate:"
            let target = alloc_obj(16, crate::nsobj::nsresponder_class_addr());
            let result =
                dispatch_u64_selector_named_to_target(target, "applicationShouldTerminate:", 0);
            assert_eq!(result, u64::MAX);
            free_obj(target);
        });

        reset_runtime_state_for_reuse();
    }
}
