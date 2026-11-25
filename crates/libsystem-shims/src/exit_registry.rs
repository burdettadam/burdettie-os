//! Launch-owned termination callbacks. There is no process-global fallback.
//!
//! A scope borrows the owner of every registered image and is confined to its
//! entering thread. Dropping it discards callbacks; it never calls guest code.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

/// The only execution route permitted for this launch's callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitRoute {
    Native,
    Arm64Interpreter,
}

/// One mapped segment, with an identity shared by segments of the same image.
#[derive(Debug, Clone)]
pub struct ExitImageRange {
    pub image: usize,
    pub start: u64,
    pub end: u64,
    pub executable: bool,
}

/// The callback's C ABI; a C++ destructor retains its actual argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCallbackAbi {
    Plain,
    Cxa,
}

/// A callback removed from the pending queue before execution.
#[derive(Debug, Clone, Copy)]
pub struct ExitCallback {
    pub address: u64,
    pub abi: ExitCallbackAbi,
    pub argument: u64,
    pub dso: u64,
    pub image: usize,
    pub dso_image: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    Normal,
    Immediate,
}

#[derive(Default)]
struct State {
    pending: Vec<ExitCallback>,
    requested: Option<(ExitKind, i32)>,
    closed: bool,
}

/// Metadata shared with an interpreter bridge, invalidated when its scope ends.
pub struct LaunchExitRegistry {
    route: ExitRoute,
    thread: ThreadId,
    ranges: Vec<ExitImageRange>,
    state: Mutex<State>,
}

thread_local! {
    static ACTIVE: RefCell<Vec<Arc<LaunchExitRegistry>>> = const { RefCell::new(Vec::new()) };
}

/// A thread-bound lifetime guard. Its destructor only discards registrations.
#[must_use = "the guard must remain alive until launch cleanup finishes"]
pub struct LaunchExitScope<'images> {
    registry: Arc<LaunchExitRegistry>,
    _images: PhantomData<&'images ()>,
    _thread_bound: PhantomData<Rc<()>>,
}

impl<'images> LaunchExitScope<'images> {
    /// Enter a launch before running any guest initializer.
    ///
    /// # Safety
    ///
    /// `owner` must keep all supplied image ranges mapped until this guard is
    /// dropped, without unloading or replacing individual images. Executable
    /// ranges must contain code for `route`; for `Native` this must match the
    /// host architecture and C ABI. Every subsequently registered native address
    /// must be a valid function entry point with the selected `ExitCallbackAbi`;
    /// being inside an executable range alone does not prove that. The caller
    /// must guarantee this for all registrations throughout the scope. Callback
    /// arguments must remain valid for their functions until execution finishes,
    /// as required by the caller of `__cxa_atexit`. This is not native CFI or a
    /// safe boundary for executing untrusted native code in the embedding host.
    pub unsafe fn enter<T: ?Sized>(
        _owner: &'images T,
        route: ExitRoute,
        ranges: Vec<ExitImageRange>,
    ) -> Self {
        let registry = Arc::new(LaunchExitRegistry {
            route,
            thread: std::thread::current().id(),
            ranges,
            state: Mutex::new(State::default()),
        });
        ACTIVE.with(|active| active.borrow_mut().push(Arc::clone(&registry)));
        Self {
            registry,
            _images: PhantomData,
            _thread_bound: PhantomData,
        }
    }

    pub fn registry(&self) -> Arc<LaunchExitRegistry> {
        Arc::clone(&self.registry)
    }
}

impl Drop for LaunchExitScope<'_> {
    fn drop(&mut self) {
        self.registry.discard();
        ACTIVE.with(|active| {
            active
                .borrow_mut()
                .retain(|entry| !Arc::ptr_eq(entry, &self.registry));
        });
    }
}

impl LaunchExitRegistry {
    /// The current thread's innermost launch. Ownership is not inherited by
    /// guest pthreads or unrelated host workers.
    pub fn current() -> Option<Arc<Self>> {
        ACTIVE.with(|active| active.borrow().last().cloned())
    }

    pub fn route(&self) -> ExitRoute {
        self.route
    }

    fn is_current(&self) -> bool {
        self.thread == std::thread::current().id()
            && ACTIVE.with(|active| {
                active
                    .borrow()
                    .last()
                    .is_some_and(|entry| std::ptr::eq(entry.as_ref(), self))
            })
    }

    /// Register only code and a DSO owned by this still-active launch.
    pub fn register(&self, address: u64, abi: ExitCallbackAbi, argument: u64, dso: u64) -> i32 {
        if !self.is_current() || address == 0 {
            return -1;
        }
        let instruction_size = match self.route {
            ExitRoute::Native => 1,
            ExitRoute::Arm64Interpreter => {
                if !address.is_multiple_of(4) {
                    return -1;
                }
                4
            }
        };
        let Some(image) = self.ranges.iter().find(|range| {
            range.executable
                && address >= range.start
                && address
                    .checked_add(instruction_size)
                    .is_some_and(|end| end <= range.end)
        }) else {
            return -1;
        };
        let dso_image = if dso == 0 {
            None
        } else {
            let Some(range) = self
                .ranges
                .iter()
                .find(|range| dso >= range.start && dso < range.end)
            else {
                return -1;
            };
            Some(range.image)
        };
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.closed {
            return -1;
        }
        state.pending.push(ExitCallback {
            address,
            abi,
            argument,
            dso,
            image: image.image,
            dso_image,
        });
        0
    }

    /// Pop in reverse registration order. `None` selects all entries; a DSO
    /// selects only that DSO. No registry lock remains held by the caller.
    pub fn take_next(&self, route: ExitRoute, dso: Option<u64>) -> Option<ExitCallback> {
        if route != self.route || !self.is_current() {
            return None;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.closed {
            return None;
        }
        let index = state
            .pending
            .iter()
            .rposition(|callback| dso.is_none_or(|handle| callback.dso == handle))?;
        Some(state.pending.remove(index))
    }

    /// Run native callbacks while the active lifetime guard still owns their
    /// mappings. Reentrant finalization cannot see an entry already removed.
    pub fn finalize_native(&self, dso: Option<u64>) {
        while let Some(callback) = self.take_next(ExitRoute::Native, dso) {
            // SAFETY: enter() establishes the image lifetime, native route and
            // ABI. register() checks the code/DSO owners; take_next() requires
            // that same scope on its owner thread and releases the queue lock.
            unsafe {
                match callback.abi {
                    ExitCallbackAbi::Plain => {
                        let function: extern "C" fn() =
                            std::mem::transmute(callback.address as usize);
                        function();
                    }
                    ExitCallbackAbi::Cxa => {
                        let function: extern "C" fn(*mut u8) =
                            std::mem::transmute(callback.address as usize);
                        function(callback.argument as *mut u8);
                    }
                }
            }
        }
    }

    /// Record an explicit exit. Immediate exit invalidates all pending entries.
    pub fn request_exit(&self, kind: ExitKind, status: i32) {
        if !self.is_current() {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.closed {
            return;
        }
        state.requested = Some((kind, status));
        if kind == ExitKind::Immediate {
            state.pending.clear();
            state.closed = true;
        }
    }

    pub fn requested_exit(&self) -> Option<(ExitKind, i32)> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .requested
    }

    /// End a launch without invoking anything, including on errors/unwinding.
    pub fn discard(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.pending.clear();
        state.closed = true;
    }
}

#[cfg(test)]
#[path = "exit_registry_tests.rs"]
mod tests;
