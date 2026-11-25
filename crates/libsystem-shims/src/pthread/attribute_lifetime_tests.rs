//! Linux-host regressions for the real pthread-create shim, not guest boot.
use super::*;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

struct Attributes {
    storage: Box<libc::pthread_attr_t>,
    initialized: bool,
}

impl Attributes {
    fn new() -> Self {
        let mut value = Self {
            // Linux shims use this stable allocation as a key, not host bytes.
            storage: Box::new(unsafe { std::mem::zeroed() }),
            initialized: false,
        };
        value.initialize();
        value
    }

    fn pointer(&self) -> *const libc::pthread_attr_t {
        std::ptr::from_ref(&*self.storage)
    }

    fn initialize(&mut self) {
        assert!(!self.initialized);
        assert_eq!(shim_pthread_attr_init(&mut *self.storage), 0);
        self.initialized = true;
    }

    fn destroy(&mut self) {
        assert!(self.initialized);
        assert_eq!(shim_pthread_attr_destroy(&mut *self.storage), 0);
        self.initialized = false;
    }

    fn set_stack_size(&mut self, size: usize) {
        assert_eq!(shim_pthread_attr_setstacksize(&mut *self.storage, size), 0);
    }

    fn stack_size(&self) -> usize {
        let mut size = 0;
        assert_eq!(shim_pthread_attr_getstacksize(self.pointer(), &mut size), 0);
        size
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        if self.initialized {
            // Cleanup must not panic a second time after a regression failure.
            let _ = shim_pthread_attr_destroy(&mut *self.storage);
        }
    }
}

struct Observation {
    attr: *const libc::pthread_attr_t,
    calls: AtomicUsize,
    getter_result: AtomicI32,
    configured_stack: AtomicUsize,
    actual_stack: AtomicUsize,
}

extern "C" fn observe_thread(raw: *mut libc::c_void) -> *mut libc::c_void {
    // SAFETY: observe() retains this allocation until a successful join.
    // The callback uses atomics and never panics across the C ABI boundary.
    let observation = unsafe { &*raw.cast::<Observation>() };
    observation.calls.fetch_add(1, Ordering::SeqCst);
    let mut size = 0;
    let rc = if observation.attr.is_null() {
        0
    } else {
        // Reenter the registry from the child: create must release its guard.
        shim_pthread_attr_getstacksize(observation.attr, &mut size)
    };
    observation.getter_result.store(rc, Ordering::SeqCst);
    observation.configured_stack.store(size, Ordering::SeqCst);
    observation.actual_stack.store(
        shim_pthread_get_stacksize_np(shim_pthread_self()),
        Ordering::SeqCst,
    );
    raw
}

#[derive(Debug)]
struct ResultRecord {
    create_result: i32,
    calls: usize,
    getter_result: i32,
    configured_stack: usize,
    actual_stack: usize,
    returned_argument: bool,
}

fn observe(attr: *const libc::pthread_attr_t) -> ResultRecord {
    let mut observation = Box::new(Observation {
        attr,
        calls: AtomicUsize::new(0),
        getter_result: AtomicI32::new(-1),
        configured_stack: AtomicUsize::new(0),
        actual_stack: AtomicUsize::new(0),
    });
    let raw = std::ptr::from_mut(&mut *observation).cast();
    let mut thread = 0;
    let rc = shim_pthread_create(&mut thread, attr, Some(observe_thread), raw);
    let mut returned = std::ptr::null_mut();
    if rc == 0 {
        let joined = shim_pthread_join(thread, &mut returned);
        if joined != 0 {
            // An unjoined child may still access its argument. Keep it alive
            // on this exceptional test failure instead of causing use-after-free.
            std::mem::forget(observation);
            panic!("pthread_join failed with {joined}");
        }
    }
    ResultRecord {
        create_result: rc,
        calls: observation.calls.load(Ordering::SeqCst),
        getter_result: observation.getter_result.load(Ordering::SeqCst),
        configured_stack: observation.configured_stack.load(Ordering::SeqCst),
        actual_stack: observation.actual_stack.load(Ordering::SeqCst),
        returned_argument: returned == raw,
    }
}

fn assert_started(record: &ResultRecord, configured: usize) {
    assert_eq!(record.create_result, 0, "{record:?}");
    assert_eq!(record.calls, 1, "{record:?}");
    assert!(record.returned_argument, "{record:?}");
    assert_eq!(record.getter_result, 0, "{record:?}");
    assert_eq!(record.configured_stack, configured, "{record:?}");
    assert!(record.actual_stack > 0, "{record:?}");
    if configured != 0 {
        // A host may reuse a larger cached stack. Check the actual thread
        // mapping as well as the exact saved attribute, without demanding
        // equality between the requested minimum and the host allocation.
        assert!(record.actual_stack >= configured, "{record:?}");
        assert!(record.actual_stack <= configured * 4, "{record:?}");
    }
}

#[test]
fn attributes_survive_repeated_creation_and_remain_mutable() {
    let mut attr = Attributes::new();
    for size in [262_144, 262_144, 524_288, 1_048_576] {
        attr.set_stack_size(size);
        assert_started(&observe(attr.pointer()), size);
        assert_eq!(attr.stack_size(), size);
        let mut detached = -1;
        assert_eq!(
            shim_pthread_attr_getdetachstate(attr.pointer(), &mut detached),
            0
        );
        assert_eq!(detached, libc::PTHREAD_CREATE_JOINABLE);
    }
}

#[test]
fn same_attributes_can_be_used_by_concurrent_creators() {
    let mut attr = Attributes::new();
    attr.set_stack_size(262_144);
    let address = attr.pointer() as usize;
    let observations = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(move || observe(address as *const libc::pthread_attr_t)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("creator panicked"))
            .collect::<Vec<_>>()
    });
    for record in &observations {
        assert_started(record, 262_144);
    }
    assert_eq!(attr.stack_size(), 262_144);
}

#[test]
fn null_attributes_still_create_a_joinable_thread() {
    assert_started(&observe(std::ptr::null()), 0);
}

#[test]
fn unregistered_nonnull_attributes_are_not_silently_defaulted() {
    let attr = Box::new(unsafe { std::mem::zeroed::<libc::pthread_attr_t>() });
    let record = observe(std::ptr::from_ref(&*attr));
    assert_eq!(record.create_result, libc::EINVAL, "{record:?}");
    assert_eq!(record.calls, 0, "{record:?}");
}

#[test]
fn destroyed_attributes_are_rejected_until_reinitialized() {
    let mut attr = Attributes::new();
    attr.destroy();
    let record = observe(attr.pointer());
    assert_eq!(record.create_result, libc::EINVAL, "{record:?}");
    assert_eq!(record.calls, 0, "{record:?}");
    attr.initialize();
    attr.set_stack_size(524_288);
    assert_started(&observe(attr.pointer()), 524_288);
}

#[test]
fn rejected_start_routine_does_not_consume_attributes() {
    let mut attr = Attributes::new();
    attr.set_stack_size(262_144);
    let mut thread = 0;
    assert_eq!(
        shim_pthread_create(&mut thread, attr.pointer(), None, std::ptr::null_mut()),
        libc::EINVAL
    );
    assert_eq!(attr.stack_size(), 262_144);
    assert_started(&observe(attr.pointer()), 262_144);
}

#[cfg(target_env = "gnu")]
#[test]
fn native_create_failure_preserves_attributes_for_retry() {
    let mut attr = Attributes::new();
    attr.set_stack_size(262_144);
    {
        let mut registry = pthread_attr_registry().lock().unwrap();
        let host = registry.get_mut(&(attr.pointer() as usize)).unwrap();
        // A zero-CPU affinity mask induces a real Linux pthread_create
        // EINVAL before the user callback, without resource exhaustion or
        // changing the test process's own affinity or scheduling policy.
        let empty = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        assert_eq!(
            unsafe {
                libc::pthread_attr_setaffinity_np(
                    &mut **host,
                    std::mem::size_of::<libc::cpu_set_t>(),
                    &empty,
                )
            },
            0
        );
    }
    let failed = observe(attr.pointer());
    assert_eq!(failed.create_result, libc::EINVAL, "{failed:?}");
    assert_eq!(failed.calls, 0, "{failed:?}");
    assert_eq!(attr.stack_size(), 262_144);
    {
        let mut inherited = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        assert_eq!(
            unsafe {
                libc::pthread_getaffinity_np(
                    shim_pthread_self(),
                    std::mem::size_of::<libc::cpu_set_t>(),
                    &mut inherited,
                )
            },
            0
        );
        let mut registry = pthread_attr_registry().lock().unwrap();
        let host = registry.get_mut(&(attr.pointer() as usize)).unwrap();
        assert_eq!(
            unsafe {
                libc::pthread_attr_setaffinity_np(
                    &mut **host,
                    std::mem::size_of::<libc::cpu_set_t>(),
                    &inherited,
                )
            },
            0
        );
    }
    assert_started(&observe(attr.pointer()), 262_144);
}
