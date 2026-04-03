//! Reference counting and autorelease pool for the lab ObjC runtime.
//!
//! Retain counts use a global `HashMap<u64, usize>` behind a `Mutex`
//! (shared across all threads — objects can be retained/released from
//! any thread). Autorelease pools are per-thread via `thread_local!`,
//! matching the real ObjC runtime where each thread has its own pool
//! stack.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn malloc_size(ptr: *const c_void) -> usize;
}

/// Global reference-counting state (shared across all threads).
pub static REFCOUNTS: Mutex<RefCountTable> = Mutex::new(RefCountTable { counts: None });

// Per-thread autorelease pool stack.
// Each thread gets its own pool stack. `objc_autoreleasePoolPush` and
// `objc_autoreleasePoolPop` operate on the calling thread's stack.
thread_local! {
    pub static AUTORELEASE_POOLS: RefCell<AutoreleasePoolStack> =
        RefCell::new(AutoreleasePoolStack::new());
}

/// Per-object reference counts.
///
/// Objects not in the table are assumed to have retain count 1
/// (the initial alloc gives an implicit +1).
pub struct RefCountTable {
    counts: Option<HashMap<u64, usize>>,
}

impl Default for RefCountTable {
    fn default() -> Self {
        Self::new()
    }
}

impl RefCountTable {
    /// Create a new empty table (for tests).
    pub fn new() -> Self {
        Self { counts: None }
    }

    fn map(&mut self) -> &mut HashMap<u64, usize> {
        self.counts.get_or_insert_with(HashMap::new)
    }

    /// Increment the retain count. Returns the new count.
    pub fn retain(&mut self, obj: u64) -> usize {
        let count = self.map().entry(obj).or_insert(1);
        *count += 1;
        *count
    }

    /// Decrement the retain count. Returns the new count.
    /// Returns 0 when the object should be deallocated.
    pub fn release(&mut self, obj: u64) -> usize {
        let map = self.map();
        let count = map.entry(obj).or_insert(1);
        if *count <= 1 {
            map.remove(&obj);
            return 0;
        }
        *count -= 1;
        *count
    }

    /// Get the current retain count (1 if not tracked).
    pub fn count(&self, obj: u64) -> usize {
        self.counts
            .as_ref()
            .and_then(|m| m.get(&obj).copied())
            .unwrap_or(1)
    }

    /// Remove tracking for an object (after dealloc).
    pub fn remove(&mut self, obj: u64) {
        if let Some(m) = self.counts.as_mut() {
            m.remove(&obj);
        }
    }

    /// Reset all state (for testing).
    pub fn reset(&mut self) {
        self.counts = None;
    }
}

pub fn object_appears_heap_allocated(obj: u64) -> bool {
    if obj == 0 {
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        unsafe { malloc_size(obj as *const c_void) != 0 }
    }

    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn deallocate_tracked_object(obj: u64, context: &str) {
    if !object_appears_heap_allocated(obj) {
        log::debug!(
            "{context}: skipping deallocation for non-malloc object {:#x}",
            obj
        );
        return;
    }

    log::debug!("{context}: deallocating {:#x}", obj);
    crate::pal_alloc().free(obj as *mut u8);
}

/// Stack of autorelease pools.
///
/// Each pool is a `Vec<u64>` of object addresses. `push()` creates a
/// new pool and returns a token. `pop(token)` drains all pools back
/// to (and including) the one identified by the token, releasing each
/// object in the pool.
pub struct AutoreleasePoolStack {
    pools: Option<Vec<Vec<u64>>>,
}

impl Default for AutoreleasePoolStack {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoreleasePoolStack {
    /// Create a new empty stack (for tests).
    pub fn new() -> Self {
        Self { pools: None }
    }

    fn stack(&mut self) -> &mut Vec<Vec<u64>> {
        self.pools.get_or_insert_with(Vec::new)
    }

    /// Push a new autorelease pool. Returns a token (pool index).
    pub fn push(&mut self) -> usize {
        let s = self.stack();
        let token = s.len();
        s.push(Vec::new());
        token
    }

    /// Add an object to the current (topmost) pool.
    /// If no pool exists, creates one implicitly.
    pub fn autorelease(&mut self, obj: u64) {
        let s = self.stack();
        if s.is_empty() {
            s.push(Vec::new());
        }
        if let Some(pool) = s.last_mut() {
            pool.push(obj);
        }
    }

    /// Pop pools back to and including the one at `token`.
    /// Returns all objects that were in the drained pools.
    pub fn pop(&mut self, token: usize) -> Vec<u64> {
        let s = self.stack();
        let mut drained = Vec::new();
        while s.len() > token {
            if let Some(pool) = s.pop() {
                drained.extend(pool);
            }
        }
        drained
    }

    /// Number of active pools.
    pub fn depth(&self) -> usize {
        self.pools.as_ref().map_or(0, |s| s.len())
    }

    /// Reset all state (for testing).
    pub fn reset(&mut self) {
        self.pools = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retain_increments() {
        let mut table = RefCountTable::new();
        assert_eq!(table.count(0x1000), 1); // implicit
        assert_eq!(table.retain(0x1000), 2);
        assert_eq!(table.retain(0x1000), 3);
        assert_eq!(table.count(0x1000), 3);
    }

    #[test]
    fn release_decrements() {
        let mut table = RefCountTable::new();
        table.retain(0x1000); // 1 → 2
        table.retain(0x1000); // 2 → 3
        assert_eq!(table.release(0x1000), 2);
        assert_eq!(table.release(0x1000), 1);
        assert_eq!(table.release(0x1000), 0); // dealloc
        assert_eq!(table.count(0x1000), 1); // removed, back to implicit
    }

    #[test]
    fn release_from_implicit_one() {
        let mut table = RefCountTable::new();
        // Object freshly alloc'd (not in table, implicit count=1)
        assert_eq!(table.release(0x2000), 0); // should dealloc
    }

    #[test]
    fn pool_push_pop() {
        let mut stack = AutoreleasePoolStack::new();
        let t1 = stack.push();
        stack.autorelease(0xA);
        stack.autorelease(0xB);

        let t2 = stack.push();
        stack.autorelease(0xC);

        assert_eq!(stack.depth(), 2);

        // Pop inner pool
        let drained = stack.pop(t2);
        assert_eq!(drained, vec![0xC]);
        assert_eq!(stack.depth(), 1);

        // Pop outer pool
        let drained = stack.pop(t1);
        assert_eq!(drained, vec![0xA, 0xB]);
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn autorelease_without_pool() {
        let mut stack = AutoreleasePoolStack::new();
        stack.autorelease(0xD); // creates implicit pool
        assert_eq!(stack.depth(), 1);
    }
}
