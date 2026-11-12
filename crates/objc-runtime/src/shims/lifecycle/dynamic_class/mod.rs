//! Dynamic class creation shims: objc_allocateClassPair,
//! objc_registerClassPair, objc_disposeClassPair, objc_duplicateClass,
//! class_addIvar, class_addProperty.

use std::collections::HashMap;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::class::ClassInfo;
use crate::MethodEntry;

// ── Synthetic address allocator ───────────────────────────────────

/// Counter for generating unique class/metaclass addresses.
/// High sentinel range avoids collision with real mapped addresses.
static NEXT_SYNTH_ADDR: AtomicU64 = AtomicU64::new(0x0050_0000_0001_0000);

fn alloc_synth_addr() -> u64 {
    // Each allocation bumps by 0x1000 (page-aligned gap) to avoid aliasing.
    NEXT_SYNTH_ADDR.fetch_add(0x1000, Ordering::Relaxed)
}

// ── Staged class pairs ────────────────────────────────────────────

struct StagedClassPair {
    class: ClassInfo,
    metaclass_addr: u64,
}

static STAGED_CLASSES: Mutex<Option<HashMap<u64, StagedClassPair>>> = Mutex::new(None);

trait StagedClassInit {
    fn insert(&mut self, key: u64, val: StagedClassPair);
    fn remove(&mut self, key: &u64) -> Option<StagedClassPair>;
    fn get_mut(&mut self, key: &u64) -> Option<&mut StagedClassPair>;
}

impl StagedClassInit for std::sync::MutexGuard<'_, Option<HashMap<u64, StagedClassPair>>> {
    fn insert(&mut self, key: u64, val: StagedClassPair) {
        self.get_or_insert_with(HashMap::new).insert(key, val);
    }
    fn remove(&mut self, key: &u64) -> Option<StagedClassPair> {
        self.as_mut()?.remove(key)
    }
    fn get_mut(&mut self, key: &u64) -> Option<&mut StagedClassPair> {
        self.as_mut()?.get_mut(key)
    }
}

fn add_method_entry(
    methods: &mut Vec<MethodEntry>,
    index: &mut HashMap<std::borrow::Cow<'static, str>, usize>,
    entry: MethodEntry,
) -> bool {
    if methods.iter().any(|m| m.selector == entry.selector) {
        return false;
    }
    let idx = methods.len();
    let selector = entry.selector.clone();
    methods.push(entry);
    index.insert(selector, idx);
    true
}

pub(crate) fn add_method_to_staged_class(cls: u64, entry: MethodEntry) -> bool {
    let mut staged = STAGED_CLASSES.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(pair) = staged.get_mut(&cls) {
        return add_method_entry(&mut pair.class.methods, &mut pair.class.method_index, entry);
    }

    if let Some((_addr, pair)) = staged.as_mut().and_then(|pairs| {
        pairs
            .iter_mut()
            .find(|(_addr, pair)| pair.metaclass_addr == cls)
    }) {
        return add_method_entry(
            &mut pair.class.class_methods,
            &mut pair.class.class_method_index,
            entry,
        );
    }

    false
}

// ── Shims ─────────────────────────────────────────────────────────

/// `Class objc_allocateClassPair(Class superclass, const char *name, size_t extraBytes)`
///
/// Creates a new class and metaclass pair. The class is not yet visible
mod class_pair_shims;
mod member_shims;

pub use class_pair_shims::*;
pub use member_shims::*;
