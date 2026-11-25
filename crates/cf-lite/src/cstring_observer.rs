//! Optional, scoped notifications to an active interpreter's string collector.
//!
//! The thread-local stack owns no string registry and holds no Memory pointer.
//! Only the central immutable constructor emits Created, while it still owns
//! the CString. Backing replacement retires any previous observation first.

use std::cell::RefCell;
use std::ffi::CStr;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum CStringObservation<'a> {
    Created { object: u64, value: &'a CStr },
    Invalidated { object: u64 },
}

pub type CStringObserver = Arc<dyn Fn(CStringObservation<'_>) + Send + Sync>;

thread_local! {
    static ACTIVE: RefCell<Vec<CStringObserver>> = const { RefCell::new(Vec::new()) };
}

/// Install an observer only for this synchronous host call. Nested calls restore
/// their predecessor even on unwind; workers do not inherit the observers.
/// Creation belongs to the innermost scope, while invalidation reaches every
/// active enclosing scope that may already have observed the same object.
pub fn with_cstring_observer<T>(observer: CStringObserver, call: impl FnOnce() -> T) -> T {
    struct Restore(usize);
    impl Drop for Restore {
        fn drop(&mut self) {
            ACTIVE.with(|active| active.borrow_mut().truncate(self.0));
        }
    }
    let _restore = Restore(ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let previous_depth = active.len();
        active.push(observer);
        previous_depth
    }));
    call()
}

fn notify(event: CStringObservation<'_>) {
    // Release the RefCell borrow before notifying; observers may nest calls.
    // An inner constructor must not grant ownership to an enclosing collector.
    match event {
        CStringObservation::Created { .. } => {
            let observer = ACTIVE.with(|active| active.borrow().last().cloned());
            if let Some(observer) = observer {
                observer(event);
            }
        }
        CStringObservation::Invalidated { .. } => {
            let observers = ACTIVE.with(|active| active.borrow().clone());
            for observer in observers {
                observer(event);
            }
        }
    }
}

pub(crate) fn observe_immutable_constructor(object: u64, value: &CStr) {
    notify(CStringObservation::Created { object, value });
}

pub(crate) fn invalidate(object: u64) {
    notify(CStringObservation::Invalidated { object });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn observer_is_nested_unwind_safe_and_not_inherited_by_workers() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let outer_seen = Arc::clone(&seen);
        let outer: CStringObserver = Arc::new(move |event| {
            outer_seen.lock().unwrap().push(match event {
                CStringObservation::Invalidated { object } => (1, object),
                CStringObservation::Created { object, .. } => (3, object),
            });
        });
        let inner_seen = Arc::clone(&seen);
        let inner: CStringObserver = Arc::new(move |event| {
            inner_seen.lock().unwrap().push(match event {
                CStringObservation::Invalidated { object } => (2, object),
                CStringObservation::Created { object, .. } => (4, object),
            });
        });
        with_cstring_observer(outer, || {
            invalidate(10);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_cstring_observer(inner, || {
                    invalidate(20);
                    observe_immutable_constructor(25, c"nested");
                    panic!("exercise observer restoration");
                });
            }));
            assert!(result.is_err());
            std::thread::spawn(|| invalidate(30)).join().unwrap();
            invalidate(40);
        });
        invalidate(50);
        assert_eq!(
            *seen.lock().unwrap(),
            [(1, 10), (1, 20), (2, 20), (4, 25), (1, 40)]
        );
    }

    #[test]
    fn constructor_reports_owned_bytes_and_reinitialization_invalidates() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let observer: CStringObserver = Arc::new(move |event| {
            captured.lock().unwrap().push(match event {
                CStringObservation::Created { object, value } => (
                    object,
                    Some((value.as_ptr() as u64, value.to_bytes().to_vec())),
                ),
                CStringObservation::Invalidated { object } => (object, None),
            });
        });
        let object = with_cstring_observer(observer, || {
            let object =
                crate::cfstring::shim_cf_string_create_with_cstring(0, c"owned".as_ptr(), 0);
            assert_ne!(object, 0);
            crate::cfstring::shim_nsstring_init_with_utf8string(object, 0, c"replacement".as_ptr());
            object
        });
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, object);
        assert_eq!(events[0].1.as_ref().unwrap().1, b"owned");
        assert_eq!(events[1], (object, None));
        // The existing runtime keeps immutable backing allocations; this test
        // does not change that lifetime contract or reconstruct a raw CString.
        crate::pal_alloc().free(object as *mut u8);
    }
}
