use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::ThreadId;

use platform_core::alloc::Allocator;
use platform_core::threading::{CondPtr, MutexPtr, Threading};

pub struct FallbackAllocator;

impl Allocator for FallbackAllocator {
    fn calloc(&self, count: usize, size: usize) -> *mut u8 {
        unsafe { libc::calloc(count, size) as *mut u8 }
    }

    fn malloc(&self, size: usize) -> *mut u8 {
        unsafe { libc::malloc(size) as *mut u8 }
    }

    fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        unsafe { libc::realloc(ptr.cast(), new_size) as *mut u8 }
    }

    fn free(&self, ptr: *mut u8) {
        unsafe { libc::free(ptr.cast()) }
    }
}

struct HostMutex {
    recursive: bool,
    state: Mutex<HostMutexState>,
    changed: Condvar,
}

#[derive(Default)]
struct HostMutexState {
    locked: bool,
    owner: Option<ThreadId>,
    recursion: usize,
}

impl HostMutex {
    fn new(recursive: bool) -> Self {
        Self {
            recursive,
            state: Mutex::new(HostMutexState::default()),
            changed: Condvar::new(),
        }
    }

    fn lock(&self) {
        let current = std::thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if self.recursive && state.owner.as_ref() == Some(&current) {
            state.recursion += 1;
            return;
        }
        while state.locked {
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        state.locked = true;
        state.owner = Some(current);
        state.recursion = 1;
    }

    fn unlock(&self) {
        let current = std::thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.owner.as_ref() != Some(&current) {
            return;
        }
        if self.recursive && state.recursion > 1 {
            state.recursion -= 1;
            return;
        }
        state.locked = false;
        state.owner = None;
        state.recursion = 0;
        self.changed.notify_one();
    }

    fn trylock(&self) -> bool {
        let current = std::thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if self.recursive && state.owner.as_ref() == Some(&current) {
            state.recursion += 1;
            return true;
        }
        if state.locked {
            return false;
        }
        state.locked = true;
        state.owner = Some(current);
        state.recursion = 1;
        true
    }
}

struct HostCond {
    generation: Mutex<u64>,
    changed: Condvar,
}

impl HostCond {
    fn new() -> Self {
        Self {
            generation: Mutex::new(0),
            changed: Condvar::new(),
        }
    }

    fn signal(&self, all: bool) {
        let mut generation = self.generation.lock().unwrap_or_else(|e| e.into_inner());
        *generation = generation.wrapping_add(1);
        if all {
            self.changed.notify_all();
        } else {
            self.changed.notify_one();
        }
    }
}

fn mutex_registry() -> &'static Mutex<HashMap<usize, Arc<HostMutex>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Arc<HostMutex>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cond_registry() -> &'static Mutex<HashMap<usize, Arc<HostCond>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Arc<HostCond>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn mutex_for(ptr: MutexPtr, recursive: bool) -> Option<Arc<HostMutex>> {
    let key = ptr as usize;
    if key == 0 {
        return None;
    }
    let mut registry = mutex_registry().lock().unwrap_or_else(|e| e.into_inner());
    Some(
        registry
            .entry(key)
            .or_insert_with(|| Arc::new(HostMutex::new(recursive)))
            .clone(),
    )
}

fn cond_for(ptr: CondPtr) -> Option<Arc<HostCond>> {
    let key = ptr as usize;
    if key == 0 {
        return None;
    }
    let mut registry = cond_registry().lock().unwrap_or_else(|e| e.into_inner());
    Some(
        registry
            .entry(key)
            .or_insert_with(|| Arc::new(HostCond::new()))
            .clone(),
    )
}

pub struct FallbackThreading;

impl Threading for FallbackThreading {
    fn mutex_size(&self) -> usize {
        std::mem::size_of::<usize>()
    }

    fn cond_size(&self) -> usize {
        std::mem::size_of::<usize>()
    }

    fn mutex_init(&self, ptr: MutexPtr) {
        let _ = mutex_for(ptr, false);
    }

    fn mutex_init_recursive(&self, ptr: MutexPtr) {
        let _ = mutex_for(ptr, true);
    }

    fn mutex_lock(&self, ptr: MutexPtr) {
        if let Some(mutex) = mutex_for(ptr, false) {
            mutex.lock();
        }
    }

    fn mutex_unlock(&self, ptr: MutexPtr) {
        if let Some(mutex) = mutex_for(ptr, false) {
            mutex.unlock();
        }
    }

    fn mutex_trylock(&self, ptr: MutexPtr) -> bool {
        mutex_for(ptr, false).is_some_and(|mutex| mutex.trylock())
    }

    fn mutex_destroy(&self, ptr: MutexPtr) {
        let key = ptr as usize;
        let mut registry = mutex_registry().lock().unwrap_or_else(|e| e.into_inner());
        registry.remove(&key);
    }

    fn cond_init(&self, ptr: CondPtr) {
        let _ = cond_for(ptr);
    }

    fn cond_wait(&self, cond: CondPtr, mutex: MutexPtr) {
        let Some(cond) = cond_for(cond) else {
            return;
        };
        let mut generation = cond.generation.lock().unwrap_or_else(|e| e.into_inner());
        let seen = *generation;
        self.mutex_unlock(mutex);
        while *generation == seen {
            generation = cond
                .changed
                .wait(generation)
                .unwrap_or_else(|e| e.into_inner());
        }
        drop(generation);
        self.mutex_lock(mutex);
    }

    fn cond_signal(&self, ptr: CondPtr) {
        if let Some(cond) = cond_for(ptr) {
            cond.signal(false);
        }
    }

    fn cond_broadcast(&self, ptr: CondPtr) {
        if let Some(cond) = cond_for(ptr) {
            cond.signal(true);
        }
    }

    fn cond_destroy(&self, ptr: CondPtr) {
        let key = ptr as usize;
        let mut registry = cond_registry().lock().unwrap_or_else(|e| e.into_inner());
        registry.remove(&key);
    }
}
