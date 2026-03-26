//! NSApplication shim: application lifecycle singleton.
//!
//! Object layout (singleton):
//!
//!   offset  0: isa            (u64 — pointer to NSApplication class)
//!   offset  8: delegate_ptr   (u64 — delegate object, or 0)
//!   offset 16: main_menu_ptr  (u64 — NSMenu object, or 0)
//!   offset 24: is_running     (u64 — 1 if run loop active, 0 otherwise)
//!
//! Total: 32 bytes.

use std::sync::OnceLock;

const DELEGATE_OFF: usize = 8;
const MAIN_MENU_OFF: usize = 16;
const IS_RUNNING_OFF: usize = 24;

static SHARED_APP: OnceLock<u64> = OnceLock::new();

fn get_or_create_shared_app() -> u64 {
    *SHARED_APP.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 32) as u64;
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
        // Allocate an 8-byte slot to hold the pointer
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
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) }
}

/// `-[NSApplication setDelegate:]`
pub extern "C" fn shim_nsapplication_set_delegate(obj: u64, _cmd: u64, delegate: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + DELEGATE_OFF) as *mut u64) = delegate; }
    log::debug!("NSApplication setDelegate: {:#x}", delegate);
}

/// `-[NSApplication mainMenu]`
pub extern "C" fn shim_nsapplication_main_menu(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + MAIN_MENU_OFF) as *const u64) }
}

/// `-[NSApplication setMainMenu:]`
pub extern "C" fn shim_nsapplication_set_main_menu(obj: u64, _cmd: u64, menu: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + MAIN_MENU_OFF) as *mut u64) = menu; }
}

/// `-[NSApplication isRunning]`
pub extern "C" fn shim_nsapplication_is_running(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
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
    if obj == 0 { return; }

    unsafe { *((obj as usize + IS_RUNNING_OFF) as *mut u64) = 1; }
    log::debug!("NSApplication run: entering run loop");

    // Notify delegate: applicationDidFinishLaunching:
    let delegate = unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) };
    if delegate != 0 {
        // Create a stub NSNotification for the launch notification
        let note = create_launch_notification();
        send_delegate_message(delegate, "applicationDidFinishLaunching:", note);
    }

    // The run loop would normally block here. For the bootstrap shim,
    // we just log and return — the fixture will call terminate: explicitly.
    log::debug!("NSApplication run: run loop iteration complete");
}

/// `-[NSApplication terminate:]`
pub extern "C" fn shim_nsapplication_terminate(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 { return; }

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

/// `-[NSApplication windows]` — returns empty NSArray.
pub extern "C" fn shim_nsapplication_windows(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

// ── Helpers ────────────────────────────────────────────────────────

fn create_launch_notification() -> u64 {
    // Create a minimal NSNotification-like object (just an NSString name).
    let name_cstr = std::ffi::CString::new("NSApplicationDidFinishLaunchingNotification").unwrap();
    crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr())
}

fn send_delegate_message(delegate: u64, selector: &str, arg: u64) {
    // Use the ObjC runtime to dispatch the message to the delegate.
    let rt_guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(ref rt) = *rt_guard {
        let isa = unsafe { *(delegate as *const u64) };
        if let Some(imp) = rt.msg_lookup(isa, selector) {
            drop(rt_guard);
            let func: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
            func(delegate, 0, arg);
        } else {
            log::debug!(
                "NSApplication: delegate {:#x} does not respond to {}",
                delegate,
                selector
            );
        }
    }
}
