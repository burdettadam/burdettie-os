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
        let lock = NOTIFICATION_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
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
    action_center: AtomicU64,
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
            action_center: AtomicU64::new(0),
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
        self.register_in(0, name, object)
    }

    fn register_in(&self, center: u64, name: u64, object: u64) -> u64 {
        let handle = shim_nsnotificationcenter_add_observer_for_name(
            center,
            0,
            name,
            object,
            0,
            self.addr(),
        );
        assert_ne!(handle, 0);
        handle
    }
}

extern "C" fn test_invoke(block: u64, notification: u64) {
    // SAFETY: tests keep this boxed TestBlock alive until synchronous delivery ends.
    let state = unsafe { &*(block as *const TestBlock) };
    state.calls.fetch_add(1, Ordering::Relaxed);
    state
        .received_name
        .store(shim_nsnotification_name(notification, 0), Ordering::Relaxed);
    state.received_object.store(
        shim_nsnotification_object(notification, 0),
        Ordering::Relaxed,
    );
    state.received_info.store(
        shim_nsnotification_userinfo(notification, 0),
        Ordering::Relaxed,
    );
    // The old lock-held implementation fails an assertion in the Rust caller
    // instead of hanging or unwinding through this extern-C callback.
    if observers().try_lock().is_err() {
        state.lock_free.store(false, Ordering::Relaxed);
        return;
    }
    let center = state.action_center.load(Ordering::Relaxed);
    let remove = state.remove.swap(0, Ordering::Relaxed);
    if remove != 0 {
        shim_nsnotificationcenter_remove_observer(center, 0, remove);
    }
    if state.clear.swap(false, Ordering::Relaxed) {
        clear_all_observers();
    }
    let add = state.add_block.swap(0, Ordering::Relaxed);
    if add != 0 {
        let handle = shim_nsnotificationcenter_add_observer_for_name(
            center,
            0,
            state.add_name.load(Ordering::Relaxed),
            0,
            0,
            add,
        );
        state.added_handle.store(handle, Ordering::Relaxed);
    }
    let repost = state.repost.swap(0, Ordering::Relaxed);
    if repost != 0 {
        shim_nsnotificationcenter_post_notification_name_object(center, 0, repost, 0);
    }
}

#[test]
fn default_center_returns_nonzero() {
    assert_ne!(shim_nsnotificationcenter_default_center(0, 0), 0);
}

#[test]
fn default_center_is_stable_singleton() {
    assert_eq!(
        shim_nsnotificationcenter_default_center(0, 0),
        default_center_addr()
    );
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
    for (posted_name, object) in [
        (other_name, sender),
        (name, other_sender),
        (name, 0),
        (name, sender),
    ] {
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
    let identities: Vec<_> = observers()
        .lock()
        .unwrap()
        .iter()
        .map(|entry| entry.matches_observer(handle))
        .collect();
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
    assert_eq!(
        shim_nsnotificationcenter_add_observer_for_name(0, 0, 0, 0, 0, 0),
        0
    );
    assert_eq!(count(), 0);
}

// The dispatcher treats the receiver as a borrowed identity. Keep an owned,
// correctly sized synthetic center alive for every registration using it.
struct TestCenter(Box<[u64; 2]>);

impl TestCenter {
    fn new() -> Self {
        Self(Box::new([nsobj::nsnotificationcenter_class_addr(), 0]))
    }

    fn addr(&self) -> u64 {
        self.0.as_ptr() as u64
    }

    fn post(&self, name: u64, object: u64) {
        shim_nsnotificationcenter_post_notification_name_object(self.addr(), 0, name, object);
    }
}

fn center_count(center: u64) -> usize {
    observers()
        .lock()
        .unwrap()
        .iter()
        .filter(|entry| entry.center == center_key(center))
        .count()
}

#[test]
fn block_delivery_is_scoped_to_its_center() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let a = TestBlock::new();
    let b = TestBlock::new();
    let default = TestBlock::new();
    let name = make_string("SeparateCenters");
    a.register_in(first.addr(), name, 0);
    b.register_in(second.addr(), name, 0);
    default.register(name, 0);
    first.post(name, 0);
    assert_eq!(a.calls.load(Ordering::Relaxed), 1);
    assert_eq!(b.calls.load(Ordering::Relaxed), 0);
    assert_eq!(default.calls.load(Ordering::Relaxed), 0);
    second.post(name, 0);
    assert_eq!(a.calls.load(Ordering::Relaxed), 1);
    assert_eq!(b.calls.load(Ordering::Relaxed), 1);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, 0);
    assert_eq!(a.calls.load(Ordering::Relaxed), 1);
    assert_eq!(b.calls.load(Ordering::Relaxed), 1);
    assert_eq!(default.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn existing_notification_delivery_uses_the_receiving_center() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let block = TestBlock::new();
    let name = make_string("ExistingNotification");
    let sender = make_string("sender");
    let info = make_string("payload");
    block.register_in(first.addr(), name, sender);
    let notification = make_nsnotification(name, sender, info);
    shim_nsnotificationcenter_post_notification(second.addr(), 0, notification);
    assert_eq!(block.calls.load(Ordering::Relaxed), 0);
    shim_nsnotificationcenter_post_notification(first.addr(), 0, notification);
    assert_eq!(block.calls.load(Ordering::Relaxed), 1);
    assert_eq!(block.received_name.load(Ordering::Relaxed), name);
    assert_eq!(block.received_object.load(Ordering::Relaxed), sender);
    assert_eq!(block.received_info.load(Ordering::Relaxed), info);
    free_obj(notification);
}

#[test]
fn selector_removal_cannot_remove_another_centers_registration() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let observer_storage = Box::new([0u64; 2]);
    let observer = observer_storage.as_ptr() as u64;
    let name = make_string("SharedObserver");
    for center in [first.addr(), second.addr()] {
        shim_nsnotificationcenter_add_observer(center, 0, observer, 1, name, 0);
    }
    shim_nsnotificationcenter_remove_observer(first.addr(), 0, observer);
    assert_eq!(center_count(first.addr()), 0);
    assert_eq!(center_count(second.addr()), 1);
    shim_nsnotificationcenter_remove_observer(second.addr(), 0, observer);
    assert_eq!(count(), 0);
}

#[test]
fn filtered_selector_removal_keeps_foreign_and_unmatched_entries() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let name = make_string("ScopedRemoval");
    let sender = make_string("sender");
    let observer_storage = Box::new([0u64; 2]);
    let observer = observer_storage.as_ptr() as u64;
    for center in [first.addr(), second.addr()] {
        shim_nsnotificationcenter_add_observer(center, 0, observer, 1, name, sender);
        shim_nsnotificationcenter_add_observer(center, 0, observer, 1, 0, 0);
    }
    shim_nsnotificationcenter_remove_observer_name_object(first.addr(), 0, observer, name, sender);
    assert_eq!(center_count(first.addr()), 1);
    assert_eq!(center_count(second.addr()), 2);
    shim_nsnotificationcenter_remove_observer_name_object(first.addr(), 0, observer, 0, 0);
    assert_eq!(center_count(first.addr()), 0);
    assert_eq!(center_count(second.addr()), 2);
}

#[test]
fn foreign_center_cannot_remove_a_block_handle() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let block = TestBlock::new();
    let name = make_string("ForeignRemoval");
    let handle = block.register_in(first.addr(), name, 0);
    shim_nsnotificationcenter_remove_observer(second.addr(), 0, handle);
    assert_eq!(count(), 1);
    first.post(name, 0);
    assert_eq!(block.calls.load(Ordering::Relaxed), 1);
    shim_nsnotificationcenter_remove_observer_name_object(second.addr(), 0, handle, name, 0);
    assert_eq!(count(), 1);
    first.post(name, 0);
    assert_eq!(block.calls.load(Ordering::Relaxed), 2);
    shim_nsnotificationcenter_remove_observer_name_object(first.addr(), 0, handle, name, 0);
    assert_eq!(count(), 0);
    free_obj(handle);
}

#[test]
fn zero_internal_center_and_default_pointer_are_aliases() {
    let _guard = RegistryGuard::new();
    let implicit = TestBlock::new();
    let explicit = TestBlock::new();
    let name = make_string("DefaultAlias");
    let implicit_handle = implicit.register(name, 0);
    let explicit_handle = explicit.register_in(default_center_addr(), name, 0);
    shim_nsnotificationcenter_post_notification_name_object(default_center_addr(), 0, name, 0);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, 0);
    assert_eq!(implicit.calls.load(Ordering::Relaxed), 2);
    assert_eq!(explicit.calls.load(Ordering::Relaxed), 2);
    shim_nsnotificationcenter_remove_observer(default_center_addr(), 0, implicit_handle);
    shim_nsnotificationcenter_remove_observer(0, 0, explicit_handle);
    assert_eq!(count(), 0);
    free_obj(implicit_handle);
    free_obj(explicit_handle);
}

#[test]
fn wildcard_registration_does_not_subscribe_to_foreign_centers() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let block = TestBlock::new();
    block.register_in(first.addr(), 0, 0);
    let name = make_string("WildcardIsStillScoped");
    let object = make_string("sender");
    second.post(name, object);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, object);
    assert_eq!(block.calls.load(Ordering::Relaxed), 0);
    first.post(name, object);
    assert_eq!(block.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn callback_foreign_removal_does_not_cancel_local_pending_delivery() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let remover = TestBlock::new();
    let pending = TestBlock::new();
    let name = make_string("PendingCenterIdentity");
    remover.register_in(first.addr(), name, 0);
    let handle = pending.register_in(first.addr(), name, 0);
    remover
        .action_center
        .store(second.addr(), Ordering::Relaxed);
    remover.remove.store(handle, Ordering::Relaxed);
    first.post(name, 0);
    assert!(remover.lock_free.load(Ordering::Relaxed));
    assert_eq!(remover.calls.load(Ordering::Relaxed), 1);
    assert_eq!(pending.calls.load(Ordering::Relaxed), 1);
    assert_eq!(center_count(first.addr()), 2);
}

#[test]
fn callback_reposts_only_to_the_requested_foreign_center() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let reposting = TestBlock::new();
    let foreign = TestBlock::new();
    let name = make_string("CrossCenterReentry");
    reposting.register_in(first.addr(), name, 0);
    foreign.register_in(second.addr(), name, 0);
    reposting
        .action_center
        .store(second.addr(), Ordering::Relaxed);
    reposting.repost.store(name, Ordering::Relaxed);
    first.post(name, 0);
    assert!(reposting.lock_free.load(Ordering::Relaxed));
    assert!(foreign.lock_free.load(Ordering::Relaxed));
    assert_eq!(reposting.calls.load(Ordering::Relaxed), 1);
    assert_eq!(foreign.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn registration_from_callback_waits_for_its_own_center() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let registering = TestBlock::new();
    let added = TestBlock::new();
    let name = make_string("RegistrationCenterIdentity");
    registering.register_in(first.addr(), name, 0);
    registering
        .action_center
        .store(second.addr(), Ordering::Relaxed);
    registering.add_block.store(added.addr(), Ordering::Relaxed);
    registering.add_name.store(name, Ordering::Relaxed);
    first.post(name, 0);
    assert!(registering.lock_free.load(Ordering::Relaxed));
    assert_ne!(registering.added_handle.load(Ordering::Relaxed), 0);
    assert_eq!(center_count(second.addr()), 1);
    first.post(name, 0);
    assert_eq!(added.calls.load(Ordering::Relaxed), 0);
    second.post(name, 0);
    assert_eq!(added.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn runtime_reset_clears_custom_and_default_centers() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let block = TestBlock::new();
    let name = make_string("ClearAllCenters");
    block.register_in(first.addr(), name, 0);
    block.register_in(second.addr(), name, 0);
    block.register(name, 0);
    assert_eq!(count(), 3);
    clear_all_observers();
    first.post(name, 0);
    second.post(name, 0);
    shim_nsnotificationcenter_post_notification_name_object(0, 0, name, 0);
    assert_eq!(block.calls.load(Ordering::Relaxed), 0);
    assert_eq!(count(), 0);
}

#[test]
fn parallel_posts_preserve_exact_per_center_delivery_counts() {
    let _guard = RegistryGuard::new();
    let first = TestCenter::new();
    let second = TestCenter::new();
    let a = TestBlock::new();
    let b = TestBlock::new();
    let default = TestBlock::new();
    let name = make_string("ParallelCenters");
    a.register_in(first.addr(), name, 0);
    b.register_in(second.addr(), name, 0);
    default.register(name, 0);
    let barrier = std::sync::Barrier::new(3);
    std::thread::scope(|scope| {
        for center in [first.addr(), second.addr(), default_center_addr()] {
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..64 {
                    shim_nsnotificationcenter_post_notification_name_object(center, 0, name, 0);
                }
            });
        }
    });
    // try_lock in the helper can legitimately observe a different posting
    // thread's lock. Only exact per-center delivery counts are asserted here.
    assert_eq!(a.calls.load(Ordering::Relaxed), 64);
    assert_eq!(b.calls.load(Ordering::Relaxed), 64);
    assert_eq!(default.calls.load(Ordering::Relaxed), 64);
}
