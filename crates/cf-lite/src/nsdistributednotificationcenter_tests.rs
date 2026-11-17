use super::*;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::MutexGuard;

static DISTRIBUTED_TEST_LOCK: Mutex<()> = Mutex::new(());

struct RegistryGuard {
    _lock: MutexGuard<'static, ()>,
}

impl RegistryGuard {
    fn new() -> Self {
        let lock = DISTRIBUTED_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_all_distributed_observers();
        Self { _lock: lock }
    }
}

impl Drop for RegistryGuard {
    fn drop(&mut self) {
        clear_all_distributed_observers();
    }
}

fn names() -> Vec<Option<String>> {
    dist_observers().lock().unwrap().iter().map(|entry| entry.name.clone()).collect()
}

struct CallbackState {
    calls: AtomicUsize,
    lock_free: AtomicBool,
    remove_self: AtomicBool,
    clear: AtomicBool,
    repost: AtomicBool,
    add_context: AtomicU64,
    args: Mutex<[u64; 5]>,
}

impl CallbackState {
    fn new() -> Box<Self> {
        Box::new(Self {
            calls: AtomicUsize::new(0),
            lock_free: AtomicBool::new(true),
            remove_self: AtomicBool::new(false),
            clear: AtomicBool::new(false),
            repost: AtomicBool::new(false),
            add_context: AtomicU64::new(0),
            args: Mutex::new([0; 5]),
        })
    }

    fn addr(&self) -> u64 {
        self as *const Self as u64
    }

    fn register(&self, name: Option<&str>) {
        add_distributed_cf_observer(name, callback as *const () as u64, self.addr());
    }
}

extern "C" fn callback(center: u64, observer: u64, name: u64, object: u64, info: u64) {
    // SAFETY: each test keeps this boxed context alive through synchronous posts.
    let state = unsafe { &*(observer as *const CallbackState) };
    state.calls.fetch_add(1, Ordering::Relaxed);
    *state.args.lock().unwrap_or_else(|e| e.into_inner()) = [center, observer, name, object, info];
    // Do not panic or hang across the C ABI when testing a lock-held regression.
    if dist_observers().try_lock().is_err() {
        state.lock_free.store(false, Ordering::Relaxed);
        return;
    }
    if state.remove_self.swap(false, Ordering::Relaxed) {
        shim_nsdistributed_remove_observer(0, 0, observer);
    }
    if state.clear.swap(false, Ordering::Relaxed) {
        clear_all_distributed_observers();
    }
    let add = state.add_context.swap(0, Ordering::Relaxed);
    if add != 0 {
        add_distributed_cf_observer(None, callback as *const () as u64, add);
    }
    if state.repost.swap(false, Ordering::Relaxed) {
        shim_nsdistributed_post_name_object_userinfo(center, 0, name, object, info);
    }
}

#[test]
fn distributed_center_returns_nonzero() {
    assert_ne!(distributed_center_ptr(), 0);
}

#[test]
fn distributed_center_is_singleton() {
    assert_eq!(distributed_center_ptr(), distributed_center_ptr());
}

#[test]
fn add_and_remove_distributed_observer() {
    let _guard = RegistryGuard::new();
    let name = cfstring::make_cfstring("TestDistNotif");
    shim_nsdistributed_add_observer(0, 0, 0x1234, 0x5678, name, 0);
    assert_eq!(names(), vec![Some("TestDistNotif".to_owned())]);
    shim_nsdistributed_remove_observer(0, 0, 0x1234);
    assert!(names().is_empty());
}

#[test]
fn remove_observer_by_name() {
    let _guard = RegistryGuard::new();
    let first = cfstring::make_cfstring("Notif1");
    let second = cfstring::make_cfstring("Notif2");
    shim_nsdistributed_add_observer(0, 0, 0xAAAA, 0xBBBB, first, 0);
    shim_nsdistributed_add_observer(0, 0, 0xAAAA, 0xBBBB, second, 0);
    assert_eq!(names().len(), 2);
    shim_nsdistributed_remove_observer_name_object(0, 0, 0xAAAA, first, 0);
    assert_eq!(names(), vec![Some("Notif2".to_owned())]);
}

#[test]
fn suspended_returns_false() {
    assert_eq!(shim_nsdistributed_suspended(0, 0), 0);
}

#[test]
fn post_with_no_observers_does_not_panic() {
    let _guard = RegistryGuard::new();
    shim_nsdistributed_post_name_object(0, 0, cfstring::make_cfstring("NoOneListening"), 0);
}

#[test]
fn cf_bridge_handles_nonzero() {
    assert_ne!(cf_distributed_center_handle(), 0);
    assert_ne!(cf_local_center_handle(), 0);
}

#[test]
fn add_observer_with_suspension_behavior() {
    let _guard = RegistryGuard::new();
    let name = cfstring::make_cfstring("TestSuspension");
    shim_nsdistributed_add_observer_suspension(0, 0, 0x1111, 0x2222, name, 0, 3);
    assert_eq!(names(), vec![Some("TestSuspension".to_owned())]);
}

#[test]
fn cf_callback_receives_all_five_arguments() {
    let _guard = RegistryGuard::new();
    let state = CallbackState::new();
    state.register(Some("CFArguments"));
    let name = cfstring::make_cfstring("CFArguments");
    let object = cfstring::make_cfstring("sender");
    let info = cfstring::make_cfstring("info");
    shim_nsdistributed_post_name_object_userinfo(0, 0, name, object, info);
    assert_eq!(state.calls.load(Ordering::Relaxed), 1);
    assert!(state.lock_free.load(Ordering::Relaxed));
    assert_eq!(*state.args.lock().unwrap(), [distributed_center_ptr(), state.addr(), name, object, info]);
    clear_all_distributed_observers();
}

#[test]
fn cf_callback_can_remove_itself_and_repost() {
    let _guard = RegistryGuard::new();
    let state = CallbackState::new();
    state.register(Some("ReentrantCF"));
    state.remove_self.store(true, Ordering::Relaxed);
    state.repost.store(true, Ordering::Relaxed);
    post_distributed_by_cstr("ReentrantCF", Some("sender"), 0);
    assert!(state.lock_free.load(Ordering::Relaxed));
    assert_eq!(state.calls.load(Ordering::Relaxed), 1);
    assert!(names().is_empty());
}

#[test]
fn reset_cancels_pending_delivery_and_defers_new_registration() {
    let _guard = RegistryGuard::new();
    let first = CallbackState::new();
    let canceled = CallbackState::new();
    let added = CallbackState::new();
    first.register(None);
    canceled.register(None);
    first.clear.store(true, Ordering::Relaxed);
    first.add_context.store(added.addr(), Ordering::Relaxed);
    post_distributed_by_cstr("ResetCF", None, 0);
    assert!(first.lock_free.load(Ordering::Relaxed));
    assert_eq!(first.calls.load(Ordering::Relaxed), 1);
    assert_eq!(canceled.calls.load(Ordering::Relaxed), 0);
    assert_eq!(added.calls.load(Ordering::Relaxed), 0);
    post_distributed_by_cstr("ResetCF", None, 0);
    assert_eq!(added.calls.load(Ordering::Relaxed), 1);
    clear_all_distributed_observers();
}

#[test]
fn cf_name_filter_and_wildcard_are_distinct() {
    let _guard = RegistryGuard::new();
    let filtered = CallbackState::new();
    let wildcard = CallbackState::new();
    filtered.register(Some("Wanted"));
    wildcard.register(None);
    post_distributed_by_cstr("Other", None, 0);
    post_distributed_by_cstr("Wanted", None, 0);
    assert_eq!(filtered.calls.load(Ordering::Relaxed), 1);
    assert_eq!(wildcard.calls.load(Ordering::Relaxed), 2);
    clear_all_distributed_observers();
}

#[test]
fn distributed_sender_filter_compares_string_contents() {
    let _guard = RegistryGuard::new();
    let state = CallbackState::new();
    // The narrow public CF helper has no sender argument. Set up the same
    // registry field used by selector registration, then exercise real delivery.
    dist_observers().lock().unwrap().push(DistObserver {
        name: Some("SenderFilter".to_owned()),
        object: Some("same sender".to_owned()),
        kind: ObserverKind::CfCallback { callback: callback as *const () as u64, context: state.addr() },
        active: Arc::new(AtomicBool::new(true)),
    });
    post_distributed_by_cstr("SenderFilter", None, 0);
    post_distributed_by_cstr("SenderFilter", Some("other sender"), 0);
    post_distributed_by_cstr("SenderFilter", Some("same sender"), 0);
    post_distributed_by_cstr("SenderFilter", Some("same sender"), 0);
    assert_eq!(state.calls.load(Ordering::Relaxed), 2);
    clear_all_distributed_observers();
}

#[test]
fn name_and_sender_removal_preserve_unrelated_and_wildcard_entries() {
    let _guard = RegistryGuard::new();
    let name = cfstring::make_cfstring("Specific");
    let object = cfstring::make_cfstring("sender");
    let equal_object = cfstring::make_cfstring("sender");
    let other = cfstring::make_cfstring("other");
    shim_nsdistributed_add_observer(0, 0, 1, 2, 0, 0);
    shim_nsdistributed_add_observer(0, 0, 1, 2, name, object);
    shim_nsdistributed_remove_observer_name_object(0, 0, 1, name, other);
    assert_eq!(names().len(), 2);
    shim_nsdistributed_remove_observer_name_object(0, 0, 1, name, equal_object);
    assert_eq!(names(), vec![None]);
    shim_nsdistributed_remove_observer_name_object(0, 0, 1, 0, 0);
    assert!(names().is_empty());
}

#[test]
fn callback_removal_does_not_remove_selectors() {
    let _guard = RegistryGuard::new();
    let state = CallbackState::new();
    state.register(None);
    shim_nsdistributed_add_observer(0, 0, 1, 2, cfstring::make_cfstring("Selector"), 0);
    remove_distributed_cf_observer(callback as *const () as u64);
    assert_eq!(names(), vec![Some("Selector".to_owned())]);
}

#[test]
fn null_callbacks_and_selectors_are_not_registered() {
    let _guard = RegistryGuard::new();
    add_distributed_cf_observer(None, 0, 0);
    shim_nsdistributed_add_observer(0, 0, 0, 1, 0, 0);
    shim_nsdistributed_add_observer(0, 0, 1, 0, 0, 0);
    assert!(names().is_empty());
}
