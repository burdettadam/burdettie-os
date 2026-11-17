use super::*;
use crate::shim_test_helpers::*;
use std::sync::atomic::AtomicUsize;
use std::sync::MutexGuard;

static NOTIFICATION_TEST_LOCK: Mutex<()> = Mutex::new(());

struct RegistryGuard {
    _lock: MutexGuard<'static, ()>,
}

impl RegistryGuard {
    fn new() -> Self {
        let lock = NOTIFICATION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_all_observers();
        Self { _lock: lock }
    }
}

impl Drop for RegistryGuard {
    fn drop(&mut self) {
        clear_all_observers();
    }
}

fn count() -> usize {
    observers().lock().unwrap().len()
}

// Real 64-bit block ABI header followed by test-owned captured state.
#[repr(C)]
struct TestBlock {
    isa: u64,
    flags: u32,
    reserved: u32,
    invoke: extern "C" fn(u64, u64),
    descriptor: u64,
    calls: AtomicUsize,
    lock_free: AtomicBool,
    remove: AtomicU64,
    clear: AtomicBool,
    repost: AtomicU64,
    add_block: AtomicU64,
    add_name: AtomicU64,
    added_handle: AtomicU64,
    received_name: AtomicU64,
    received_object: AtomicU64,
    received_info: AtomicU64,
}

impl TestBlock {
    fn new() -> Box<Self> {
        Box::new(Self {
            isa: 0,
            flags: 0,
            reserved: 0,
            invoke: test_invoke,
            descriptor: 0,
            calls: AtomicUsize::new(0),
            lock_free: AtomicBool::new(true),
            remove: AtomicU64::new(0),
            clear: AtomicBool::new(false),
            repost: AtomicU64::new(0),
            add_block: AtomicU64::new(0),
            add_name: AtomicU64::new(0),
            added_handle: AtomicU64::new(0),
            received_name: AtomicU64::new(0),
            received_object: AtomicU64::new(0),
            received_info: AtomicU64::new(0),
        })
    }

    fn addr(&self) -> u64 {
        self as *const Self as u64
    }

    fn register(&self, name: u64, object: u64) -> u64 {
        let handle = shim_nsnotificationcenter_add_observer_for_name(0, 0, name, object, 0, self.addr());
        assert_ne!(handle, 0);
        handle
    }
}

extern "C" fn test_invoke(block: u64, notification: u64) {
    // SAFETY: tests keep this boxed TestBlock alive until synchronous delivery ends.
    let state = unsafe { &*(block as *const TestBlock) };
    state.calls.fetch_add(1, Ordering::Relaxed);
    state.received_name.store(shim_nsnotification_name(notification, 0), Ordering::Relaxed);
    state.received_object.store(shim_nsnotification_object(notification, 0), Ordering::Relaxed);
    state.received_info.store(shim_nsnotification_userinfo(notification, 0), Ordering::Relaxed);
    // The old lock-held implementation fails an assertion in the Rust caller
    // instead of hanging or unwinding through this extern-C callback.
    if observers().try_lock().is_err() {
        state.lock_free.store(false, Ordering::Relaxed);
        return;
    }
    let remove = state.remove.swap(0, Ordering::Relaxed);
    if remove != 0 {
        shim_nsnotificationcenter_remove_observer(0, 0, remove);
    }
    if state.clear.swap(false, Ordering::Relaxed) {
        clear_all_observers();
    }
    let add = state.add_block.swap(0, Ordering::Relaxed);
    if add != 0 {
        let handle = shim_nsnotificationcenter_add_observer_for_name(
            0, 0, state.add_name.load(Ordering::Relaxed), 0, 0, add,
        );
        state.added_handle.store(handle, Ordering::Relaxed);
    }
    let repost = state.repost.swap(0, Ordering::Relaxed);
    if repost != 0 {
        shim_nsnotificationcenter_post_notification_name_object(0, 0, repost, 0);
    }
}

#[test]
fn default_center_returns_nonzero() {
    assert_ne!(shim_nsnotificationcenter_default_center(0, 0), 0);
}

#[test]
fn default_center_is_stable_singleton() {
    assert_eq!(shim_nsnotificationcenter_default_center(0, 0), default_center_addr());
}

#[test]
fn make_notification_stores_fields() {
    let name = make_string("MyNotification");
    let sender = make_string("sender-object");
    let notification = make_nsnotification(name, sender, 0);
    assert_ne!(notification, 0);
    assert_eq!(shim_nsnotification_name(notification, 0), name);
    assert_eq!(shim_nsnotification_object(notification, 0), sender);
    assert_eq!(shim_nsnotification_userinfo(notification, 0), 0);
    free_obj(notification);
}

#[test]
fn notification_with_name_object_userinfo_shim() {
    let name = make_string("TestNotifFull");
    let info = make_string("extra-info");
    let notification = shim_nsnotification_with_name_object_userinfo(0, 0, name, 0, info);
    assert_ne!(notification, 0);
    assert_eq!(shim_nsnotification_name(notification, 0), name);
    assert_eq!(shim_nsnotification_userinfo(notification, 0), info);
    free_obj(notification);
}

#[test]
fn notification_with_name_object_shim_no_userinfo() {
    let name = make_string("TestNotifSimple");
    let notification = shim_nsnotification_with_name_object(0, 0, name, 0);
    assert_ne!(notification, 0);
    assert_eq!(shim_nsnotification_name(notification, 0), name);
    assert_eq!(shim_nsnotification_userinfo(notification, 0), 0);
    free_obj(notification);
}

#[test]
fn notification_accessors_nil_safety() {
    assert_eq!(shim_nsnotification_name(0, 0), 0);
    assert_eq!(shim_nsnotification_object(0, 0), 0);
    assert_eq!(shim_nsnotification_userinfo(0, 0), 0);
}

#[test]
fn remove_observer_nil_is_noop() {
    let _guard = RegistryGuard::new();
    shim_nsnotificationcenter_remove_observer(default_center_addr(), 0, 0);
    assert_eq!(count(), 0);
}

#[test]
fn post_notification_with_nil_name_does_not_crash() {
    let _guard = RegistryGuard::new();
    shim_nsnotificationcenter_post_notification_name_object(default_center_addr(), 0, 0, 0);
    shim_nsnotificationcenter_post_notification(0, 0, 0);
}

#[test]
fn clear_all_observers_resets_registry_and_token_counter() {
    let _guard = RegistryGuard::new();
    let block = TestBlock::new();
    let first = block.register(make_string("Alpha"), 0);
    let second = block.register(make_string("Beta"), 0);
    assert_eq!(count(), 2);
    NEXT_TOKEN.store(99, Ordering::Relaxed);
    clear_all_observers();
    assert_eq!(count(), 0);
    assert_eq!(NEXT_TOKEN.load(Ordering::Relaxed), 1);
    free_obj(first);
    free_obj(second);
}

#[test]
fn block_callback_receives_notification_fields() {
    let _guard = RegistryGuard::new();
    let block = TestBlock::new();
    let name = make_string("CallbackFields");
    let object = make_string("sender");
    let info = make_string("info");
    block.register(name, object);
    shim_nsnotificationcenter_post_notification_name_object_userinfo(0, 0, name, object, info);
    assert_eq!(block.calls.load(Ordering::Relaxed), 1);
    assert!(block.lock_free.load(Ordering::Relaxed));
    assert_eq!(block.received_name.load(Ordering::Relaxed), name);
    assert_eq!(block.received_object.load(Ordering::Relaxed), object);
    assert_eq!(block.received_info.load(Ordering::Relaxed), info);
    clear_all_observers();
}

#[test]
fn callback_can_remove_itself_and_repost_without_deadlock() {
    let _guard = RegistryGuard::new();
    let block = TestBlock::new();
    let name = make_string("ReentrantRemoval");
    let handle = block.register(name, 0);
    block.remove.store(handle, Ordering::Relaxed);
    block.repost.store(name, Ordering::Relaxed);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, 0);
    assert!(block.lock_free.load(Ordering::Relaxed));
    assert_eq!(block.calls.load(Ordering::Relaxed), 1);
    assert_eq!(count(), 0);
    free_obj(handle);
}

#[test]
fn removed_pending_observer_is_not_invoked() {
    let _guard = RegistryGuard::new();
    let first = TestBlock::new();
    let second = TestBlock::new();
    let name = make_string("CancelPending");
    first.register(name, 0);
    let second_handle = second.register(name, 0);
    first.remove.store(second_handle, Ordering::Relaxed);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, 0);
    assert!(first.lock_free.load(Ordering::Relaxed));
    assert_eq!(first.calls.load(Ordering::Relaxed), 1);
    assert_eq!(second.calls.load(Ordering::Relaxed), 0);
    clear_all_observers();
}

#[test]
fn reset_and_registration_do_not_reuse_pending_delivery() {
    let _guard = RegistryGuard::new();
    let first = TestBlock::new();
    let canceled = TestBlock::new();
    let added = TestBlock::new();
    let name = make_string("ResetDuringDelivery");
    first.register(name, 0);
    canceled.register(name, 0);
    first.clear.store(true, Ordering::Relaxed);
    first.add_block.store(added.addr(), Ordering::Relaxed);
    first.add_name.store(name, Ordering::Relaxed);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, 0);
    assert!(first.lock_free.load(Ordering::Relaxed));
    assert_ne!(first.added_handle.load(Ordering::Relaxed), 0);
    assert_eq!(canceled.calls.load(Ordering::Relaxed), 0);
    assert_eq!(added.calls.load(Ordering::Relaxed), 0);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, 0);
    assert_eq!(added.calls.load(Ordering::Relaxed), 1);
    clear_all_observers();
}

#[test]
fn sender_and_name_filters_apply_to_delivery() {
    let _guard = RegistryGuard::new();
    let filtered = TestBlock::new();
    let wildcard = TestBlock::new();
    let name = make_string("Filtered");
    let other_name = make_string("Other");
    let sender = make_string("sender");
    let other_sender = make_string("sender");
    filtered.register(name, sender);
    wildcard.register(0, 0);
    for (posted_name, object) in [(other_name, sender), (name, other_sender), (name, 0), (name, sender)] {
        shim_nsnotificationcenter_post_notification_name_object(0, 0, posted_name, object);
    }
    assert_eq!(filtered.calls.load(Ordering::Relaxed), 1);
    assert_eq!(wildcard.calls.load(Ordering::Relaxed), 4);
    clear_all_observers();
}

#[test]
fn named_removal_honors_sender_and_supports_block_handles() {
    let _guard = RegistryGuard::new();
    let block = TestBlock::new();
    let name = make_string("NamedRemoval");
    let sender = make_string("sender");
    let other = make_string("other");
    let handle = block.register(name, sender);
    shim_nsnotificationcenter_remove_observer_name_object(0, 0, handle, name, other);
    assert_eq!(count(), 1);
    shim_nsnotificationcenter_remove_observer_name_object(0, 0, handle, name, sender);
    assert_eq!(count(), 0);
    free_obj(handle);
}

#[test]
fn specific_name_removal_preserves_wildcard_registration() {
    let _guard = RegistryGuard::new();
    let block = TestBlock::new();
    let handle = block.register(0, 0);
    shim_nsnotificationcenter_remove_observer_name_object(0, 0, handle, make_string("Specific"), 0);
    assert_eq!(count(), 1);
    shim_nsnotificationcenter_remove_observer_name_object(0, 0, handle, 0, 0);
    assert_eq!(count(), 0);
    free_obj(handle);
}

#[test]
fn selector_ivar_that_looks_like_a_token_does_not_remove_block() {
    let _guard = RegistryGuard::new();
    let block = TestBlock::new();
    let name = make_string("IdentityNotIvar");
    let handle = block.register(name, 0);
    let selector_object = Box::new([0u64, 1u64]);
    let observer = selector_object.as_ptr() as u64;
    shim_nsnotificationcenter_add_observer(0, 0, observer, 1, name, 0);
    shim_nsnotificationcenter_remove_observer(0, 0, observer);
    let identities: Vec<_> = observers().lock().unwrap().iter().map(|entry| entry.matches_observer(handle)).collect();
    assert_eq!(identities, vec![true]);
    clear_all_observers();
    free_obj(handle);
}

#[test]
fn stale_block_handle_cannot_remove_new_registration_after_reset() {
    let _guard = RegistryGuard::new();
    let block = TestBlock::new();
    let old = block.register(0, 0);
    clear_all_observers();
    let new = block.register(0, 0);
    assert_ne!(old, new);
    shim_nsnotificationcenter_remove_observer(0, 0, old);
    assert_eq!(count(), 1);
    shim_nsnotificationcenter_remove_observer(0, 0, new);
    assert_eq!(count(), 0);
    free_obj(old);
    free_obj(new);
}

#[test]
fn nil_observers_and_blocks_are_not_registered() {
    let _guard = RegistryGuard::new();
    shim_nsnotificationcenter_add_observer(0, 0, 0, 1, 0, 0);
    shim_nsnotificationcenter_add_observer(0, 0, 1, 0, 0, 0);
    assert_eq!(shim_nsnotificationcenter_add_observer_for_name(0, 0, 0, 0, 0, 0), 0);
    assert_eq!(count(), 0);
}
