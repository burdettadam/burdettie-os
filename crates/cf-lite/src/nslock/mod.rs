//! NSLock, NSRecursiveLock, NSCondition, NSConditionLock shims.
//!
//! These Foundation classes wrap POSIX threading primitives.
//! Since our pthread shims delegate to the host OS, these classes
//! provide real locking with full thread safety.
//!
//! ## Object Layouts
//!
//! **NSLock** (80 bytes):
//!   - [0..8)    ISA pointer
//!   - [8..72)   pthread_mutex_t (64 bytes on arm64 macOS)
//!   - [72..80)  name pointer (NSString * or 0)
//!
//! **NSRecursiveLock** (80 bytes): same layout, recursive mutex type.
//!
//! **NSCondition** (128 bytes):
//!   - [0..8)     ISA pointer
//!   - [8..72)    pthread_mutex_t (64 bytes)
//!   - [72..120)  pthread_cond_t (48 bytes)
//!   - [120..128) name pointer
//!
//! **NSConditionLock** (136 bytes):
//!   - [0..8)     ISA pointer
//!   - [8..72)    pthread_mutex_t (64 bytes)
//!   - [72..120)  pthread_cond_t (48 bytes)
//!   - [120..128) condition value (i64)
//!   - [128..136) name pointer

mod condition;
mod conditionlock;
mod lock;
mod recursive;

pub use condition::*;
pub use conditionlock::*;
pub use lock::*;
pub use recursive::*;

use crate::cfstring;
use crate::nsobj;

pub(super) const NSLOCK_SIZE: usize = 80;
pub(super) const NSRECURSIVELOCK_SIZE: usize = 80;
pub(super) const NSCONDITION_SIZE: usize = 128;
pub(super) const NSCONDITIONLOCK_SIZE: usize = 136;

// Offsets
pub(super) const MUTEX_OFFSET: usize = 8;
pub(super) const NSLOCK_NAME_OFFSET: usize = 72;
pub(super) const COND_OFFSET: usize = 72;
pub(super) const NSCONDITION_NAME_OFFSET: usize = 120;
pub(super) const CONDLOCK_VALUE_OFFSET: usize = 120;
pub(super) const CONDLOCK_NAME_OFFSET: usize = 128;

pub(super) fn mutex_ptr(obj: u64) -> *mut u8 {
    (obj as usize + MUTEX_OFFSET) as *mut u8
}

pub(super) fn cond_ptr(obj: u64) -> *mut u8 {
    (obj as usize + COND_OFFSET) as *mut u8
}
