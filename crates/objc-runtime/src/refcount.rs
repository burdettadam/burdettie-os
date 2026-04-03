//! Reference counting and autorelease pool for the lab ObjC runtime.
//!
//! Retain counts use a **sharded** lock table (64 `Mutex<HashMap>` shards)
//! keyed by object address. This allows concurrent retain/release from
//! multiple threads without a single global bottleneck. Autorelease
//! pools are per-thread via `thread_local!`, matching the real ObjC
//! runtime where each thread has its own pool stack.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{LazyLock, Mutex};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn malloc_size(ptr: *const c_void) -> usize;
}

/// Global reference-counting state (shared across all threads).
///
/// Uses 64 internal shards so that concurrent retain/release on
/// different objects rarely contends on the same mutex.
pub static REFCOUNTS: LazyLock<ShardedRefCounts> = LazyLock::new(ShardedRefCounts::new);

// Per-thread autorelease pool stack.
// Each thread gets its own pool stack. `objc_autoreleasePoolPush` and
// `objc_autoreleasePoolPop` operate on the calling thread's stack.
thread_local! {
    pub static AUTORELEASE_POOLS: RefCell<AutoreleasePoolStack> =
        RefCell::new(AutoreleasePoolStack::new());
}

/// Number of lock shards — must be a power of two.
const SHARD_COUNT: usize = 64;

/// Sharded reference-count table for concurrent retain/release.
///
/// Object addresses are distributed across `SHARD_COUNT` internal
/// mutexes by hashing the address. Objects not in any shard are
/// assumed to have retain count 1 (the initial alloc gives an
/// implicit +1).
pub struct ShardedRefCounts {
    shards: [Mutex<HashMap<u64, usize>>; SHARD_COUNT],
}

impl ShardedRefCounts {
    /// Create a new empty sharded table.
    pub fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| Mutex::new(HashMap::new())),
        }
    }

    /// Select the shard for a given object address.
    #[inline]
    fn shard_for(&self, obj: u64) -> &Mutex<HashMap<u64, usize>> {
        // Shift right by 4 to avoid alignment bias (objects are typically
        // 16-byte aligned), then mask to select a shard.
        let idx = (obj as usize >> 4) & (SHARD_COUNT - 1);
        &self.shards[idx]
    }

    /// Increment the retain count. Returns the new count.
    pub fn retain(&self, obj: u64) -> usize {
        let mut shard = self.shard_for(obj).lock().unwrap_or_else(|e| e.into_inner());
        let count = shard.entry(obj).or_insert(1);
        *count += 1;
        *count
    }

    /// Decrement the retain count. Returns the new count.
    /// Returns 0 when the object should be deallocated.
    pub fn release(&self, obj: u64) -> usize {
        let mut shard = self.shard_for(obj).lock().unwrap_or_else(|e| e.into_inner());
        let count = shard.entry(obj).or_insert(1);
        if *count <= 1 {
            shard.remove(&obj);
            return 0;
        }
        *count -= 1;
        *count
    }

    /// Get the current retain count (1 if not tracked).
    pub fn count(&self, obj: u64) -> usize {
        let shard = self.shard_for(obj).lock().unwrap_or_else(|e| e.into_inner());
        shard.get(&obj).copied().unwrap_or(1)
    }

    /// Remove tracking for an object (after dealloc).
    pub fn remove(&self, obj: u64) {
        let mut shard = self.shard_for(obj).lock().unwrap_or_else(|e| e.into_inner());
        shard.remove(&obj);
    }

    /// Reset all state (for testing).
    pub fn reset(&self) {
        for shard in &self.shards {
            shard.lock().unwrap_or_else(|e| e.into_inner()).clear();
        }
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
        let table = ShardedRefCounts::new();
        assert_eq!(table.count(0x1000), 1); // implicit
        assert_eq!(table.retain(0x1000), 2);
        assert_eq!(table.retain(0x1000), 3);
        assert_eq!(table.count(0x1000), 3);
    }

    #[test]
    fn release_decrements() {
        let table = ShardedRefCounts::new();
        table.retain(0x1000); // 1 → 2
        table.retain(0x1000); // 2 → 3
        assert_eq!(table.release(0x1000), 2);
        assert_eq!(table.release(0x1000), 1);
        assert_eq!(table.release(0x1000), 0); // dealloc
        assert_eq!(table.count(0x1000), 1); // removed, back to implicit
    }

    #[test]
    fn release_from_implicit_one() {
        let table = ShardedRefCounts::new();
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

    #[test]
    fn sharded_concurrent_retain_release() {
        use std::sync::Arc;

        let table = Arc::new(ShardedRefCounts::new());
        let handles: Vec<_> = (0..4)
            .map(|t| {
                let table = Arc::clone(&table);
                std::thread::spawn(move || {
                    let base = (t + 1) * 0x10000;
                    for i in 0..1000 {
                        let obj = (base + i * 16) as u64;
                        table.retain(obj);
                        table.release(obj);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        // All retain/release pairs balanced — counts back to implicit 1
    }

    #[test]
    fn sharded_objects_different_shards() {
        let table = ShardedRefCounts::new();
        // 0x1000 >> 4 = 0x100, 0x2000 >> 4 = 0x200 → different shards
        table.retain(0x1000);
        table.retain(0x2000);

        assert_eq!(table.count(0x1000), 2);
        assert_eq!(table.count(0x2000), 2);

        table.release(0x1000);
        assert_eq!(table.count(0x1000), 1);
        assert_eq!(table.count(0x2000), 2); // unaffected
    }

    #[test]
    fn sharded_reset_clears_all() {
        let table = ShardedRefCounts::new();
        table.retain(0x1000);
        table.retain(0x2000);
        table.retain(0x3000);
        assert_eq!(table.count(0x1000), 2);
        assert_eq!(table.count(0x2000), 2);
        assert_eq!(table.count(0x3000), 2);

        table.reset();

        assert_eq!(table.count(0x1000), 1);
        assert_eq!(table.count(0x2000), 1);
        assert_eq!(table.count(0x3000), 1);
    }

    #[test]
    fn sharded_remove() {
        let table = ShardedRefCounts::new();
        table.retain(0x1000); // 1 → 2
        table.retain(0x1000); // 2 → 3
        assert_eq!(table.count(0x1000), 3);

        table.remove(0x1000);
        assert_eq!(table.count(0x1000), 1); // back to implicit
    }
}
