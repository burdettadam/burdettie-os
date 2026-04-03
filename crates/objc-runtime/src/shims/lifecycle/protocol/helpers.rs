//! Shared helpers for protocol shims: staged storage, name-to-sentinel mapping.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::class::ProtocolInfo;

/// Protocols that have been allocated but not yet registered.
pub(super) static STAGED_PROTOCOLS: Mutex<Option<HashMap<u64, ProtocolInfo>>> = Mutex::new(None);

/// Trait to initialize-once and access staged protocols.
pub(super) trait StagedInit {
    fn insert(&mut self, key: u64, val: ProtocolInfo);
    fn remove(&mut self, key: &u64) -> Option<ProtocolInfo>;
}

impl StagedInit for std::sync::MutexGuard<'_, Option<HashMap<u64, ProtocolInfo>>> {
    fn insert(&mut self, key: u64, val: ProtocolInfo) {
        self.get_or_insert_with(HashMap::new).insert(key, val);
    }
    fn remove(&mut self, key: &u64) -> Option<ProtocolInfo> {
        self.as_mut()?.remove(key)
    }
}

// ── Helpers ────────────────────────────────────────────────────────

pub(super) fn protocol_name_to_sentinel(name: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h | 0x7000_0000_0000_0000
}

pub(super) fn sentinel_to_protocol_name(
    rt: &crate::class::ObjcRuntime,
    sentinel: u64,
) -> Option<String> {
    for (name, _) in rt.protocols_iter() {
        if protocol_name_to_sentinel(name) == sentinel {
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::ObjcRuntime;

    #[test]
    fn sentinel_has_high_bits() {
        let s = protocol_name_to_sentinel("NSCoding");
        assert_eq!(s & 0x7000_0000_0000_0000, 0x7000_0000_0000_0000);
    }

    #[test]
    fn sentinel_deterministic() {
        let a = protocol_name_to_sentinel("NSCopying");
        let b = protocol_name_to_sentinel("NSCopying");
        assert_eq!(a, b);
    }

    #[test]
    fn sentinel_different_names() {
        let a = protocol_name_to_sentinel("NSCoding");
        let b = protocol_name_to_sentinel("NSCopying");
        assert_ne!(a, b);
    }

    #[test]
    fn sentinel_nonempty() {
        let s = protocol_name_to_sentinel("SomeProto");
        assert_ne!(s, 0);
    }

    #[test]
    fn reverse_lookup_without_runtime() {
        let rt = ObjcRuntime::new();
        let s = protocol_name_to_sentinel("NoSuchProto");
        assert!(sentinel_to_protocol_name(&rt, s).is_none());
    }
}
