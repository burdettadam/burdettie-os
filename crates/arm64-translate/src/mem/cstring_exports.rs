//! Owned snapshots of immutable strings observed during this interpreter.
//!
//! These are data-copy capabilities, not general memory/code mappings. Creating
//! a record does not export it: the runtime must also observe the exact known
//! UTF8String implementation returning the recorded backing address.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::sync::{Arc, Mutex};

pub const MAX_EXPORTED_CSTRING_BYTES: usize = 4096;
const MAX_OBSERVED_STRINGS: usize = 1024;
const MAX_OBSERVED_BYTES: usize = 1024 * 1024;

#[derive(Default)]
struct State {
    records: BTreeMap<u64, Record>,
    exported: BTreeMap<u64, Arc<CStr>>,
    bytes: usize,
}

struct Record {
    address: u64,
    value: Arc<CStr>,
}

/// A collector owned by one Memory, temporarily shared with a scoped observer.
/// It never dereferences object IDs or backing addresses supplied as integers.
#[derive(Clone, Default)]
pub struct CStringExports(Arc<Mutex<State>>);

impl CStringExports {
    /// Observe a constructor's live, Rust-owned C string before `into_raw`.
    /// The caller must supply only the approved immutable constructor's data.
    /// Unknown/oversized inputs are not recorded and cannot later be exported.
    pub fn observe_immutable(&self, object: u64, value: &CStr) -> bool {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        Self::invalidate_locked(&mut state, object);
        let length = value.to_bytes_with_nul().len();
        let address = value.as_ptr() as u64;
        if object == 0
            || value.to_bytes().len() > MAX_EXPORTED_CSTRING_BYTES
            || state.records.len() >= MAX_OBSERVED_STRINGS
            || length > MAX_OBSERVED_BYTES - state.bytes
            || address.checked_add(length as u64).is_none()
        {
            return false;
        }
        state.bytes += length;
        state.records.insert(
            object,
            Record {
                address,
                value: Arc::from(value),
            },
        );
        true
    }

    /// Retire both pending and exported snapshots before a backing replacement.
    pub fn invalidate(&self, object: u64) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        Self::invalidate_locked(&mut state, object);
    }

    fn invalidate_locked(state: &mut State, object: u64) {
        if let Some(record) = state.records.remove(&object) {
            state.bytes -= record.value.to_bytes_with_nul().len();
            state.exported.remove(&record.address);
        }
    }

    /// Approve only the backing address returned by the known getter for this
    /// exact observed object. No receiver fields are inspected here.
    pub(crate) fn export(&self, object: u64, address: u64) -> bool {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let Some(record) = state.records.get(&object) else {
            return false;
        };
        if record.address != address {
            return false;
        }
        let value = Arc::clone(&record.value);
        state.exported.insert(address, value);
        true
    }

    pub(crate) fn copy_cstring(&self, address: u64, limit: usize) -> Option<Vec<u8>> {
        let state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let (&base, value) = state.exported.range(..=address).next_back()?;
        let offset = usize::try_from(address.checked_sub(base)?).ok()?;
        let bytes = value.to_bytes_with_nul().get(offset..)?;
        // The final NUL is part of the snapshot. Interior NULs cannot exist.
        let content = bytes.strip_suffix(&[0])?;
        (content.len() <= limit).then(|| content.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::{BoundedReadError, Memory};
    use std::ffi::CString;

    #[test]
    fn export_requires_observation_identity_and_keeps_owned_bytes() {
        let exports = CStringExports::default();
        let value = CString::new("hello").unwrap();
        let address = value.as_ptr() as u64;
        assert!(exports.observe_immutable(7, &value));
        assert!(exports.copy_cstring(address, 5).is_none());
        assert!(!exports.export(8, address));
        assert!(!exports.export(7, address + 1));
        assert!(exports.export(7, address));
        drop(value);
        // Copying uses the owned snapshot, not the now-invalid source address.
        assert_eq!(exports.copy_cstring(address, 5), Some(b"hello".to_vec()));
        assert_eq!(exports.copy_cstring(address + 2, 3), Some(b"llo".to_vec()));
        assert_eq!(exports.copy_cstring(address + 5, 0), Some(Vec::new()));
        assert!(exports.copy_cstring(address, 4).is_none());
        assert!(exports.copy_cstring(address + 6, 20).is_none());
        exports.invalidate(7);
        assert!(exports.copy_cstring(address, 5).is_none());
        assert!(!exports.export(7, address));
    }

    #[test]
    fn observation_limits_and_reinitialization_retire_old_export() {
        let exports = CStringExports::default();
        assert!(!exports.observe_immutable(0, c"zero"));
        let oversized = CString::new(vec![b'x'; MAX_EXPORTED_CSTRING_BYTES + 1]).unwrap();
        assert!(!exports.observe_immutable(1, &oversized));
        assert!(exports.observe_immutable(1, c"first"));
        assert!(exports.export(1, c"first".as_ptr() as u64));
        assert!(exports.observe_immutable(1, c"second"));
        assert!(exports.copy_cstring(c"first".as_ptr() as u64, 5).is_none());
        assert!(!exports.export(1, c"first".as_ptr() as u64));

        let large = CString::new(vec![b'x'; MAX_EXPORTED_CSTRING_BYTES]).unwrap();
        for id in 2..257 {
            assert!(exports.observe_immutable(id, &large));
        }
        assert!(!exports.observe_immutable(257, &large));
        exports.invalidate(2);
        assert!(exports.observe_immutable(257, &large));

        let count_limited = CStringExports::default();
        for id in 1..=MAX_OBSERVED_STRINGS as u64 {
            assert!(count_limited.observe_immutable(id, c""));
        }
        assert!(!count_limited.observe_immutable(MAX_OBSERVED_STRINGS as u64 + 1, c""));
    }

    #[test]
    fn strict_byte_reads_never_use_host_passthrough_or_export_code() {
        let bytes = [b'a', 0xff, 0, b'x'];
        // SAFETY: the read-only array stays live until Memory is dropped.
        let mut memory = unsafe {
            Memory::from_segments(&[(0x4000, bytes.as_ptr().cast_mut(), bytes.len(), false)])
        };
        memory.host_passthrough = true;
        assert_eq!(memory.copy_cstring_bounded(0x4000, 2), Ok(vec![b'a', 0xff]));
        assert_eq!(
            memory.copy_cstring_bounded(0x4000, 1),
            Err(BoundedReadError::MissingTerminator)
        );
        assert_eq!(
            memory.copy_cstring_bounded(0x4003, 8),
            Err(BoundedReadError::Unmapped)
        );
        let owned = CString::new("owned").unwrap();
        let address = owned.as_ptr() as u64;
        assert!(memory.copy_cstring_bounded(address, 5).is_err());
        assert!(memory.cstring_exports().observe_immutable(9, &owned));
        assert!(memory.copy_cstring_bounded(address, 5).is_err());
        assert!(memory.export_observed_cstring(9, address));
        assert_eq!(
            memory.copy_cstring_bounded(address, 5),
            Ok(b"owned".to_vec())
        );
        assert!(
            !memory.is_valid(address),
            "exports must not become guest code"
        );
        assert!(memory.read_u64_mapped(address).is_none());
        let weak = Arc::downgrade(&memory.cstring_exports.0);
        drop(memory);
        assert!(
            weak.upgrade().is_none(),
            "the interpreter owns the collector"
        );
    }

    #[test]
    fn bounded_strings_reject_address_wraparound() {
        let bytes = [b'x', b'y'];
        // SAFETY: both last-address bytes are backed by this live array.
        let memory = unsafe {
            Memory::from_segments(&[(u64::MAX - 1, bytes.as_ptr().cast_mut(), 2, false)])
        };
        assert_eq!(
            memory.copy_cstring_bounded(u64::MAX - 1, 2),
            Err(BoundedReadError::AddressOverflow)
        );
    }
}
