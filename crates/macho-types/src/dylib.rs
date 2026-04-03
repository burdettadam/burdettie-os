use serde::{Deserialize, Serialize};

/// Reference to a dynamic library from LC_LOAD_DYLIB / LC_ID_DYLIB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DylibRef {
    /// Install name path (may contain @rpath, @loader_path, @executable_path).
    pub name: String,
    /// Compatibility version encoded as u32 (major.minor.patch).
    pub compatibility_version: u32,
    /// Current version encoded as u32.
    pub current_version: u32,
    /// Timestamp (usually build time, often 2 = "not set").
    pub timestamp: u32,
}

impl DylibRef {
    /// Decode a packed version u32 into (major, minor, patch).
    pub fn decode_version(v: u32) -> (u16, u8, u8) {
        let major = (v >> 16) as u16;
        let minor = ((v >> 8) & 0xFF) as u8;
        let patch = (v & 0xFF) as u8;
        (major, minor, patch)
    }

    pub fn current_version_string(&self) -> String {
        let (maj, min, pat) = Self::decode_version(self.current_version);
        format!("{maj}.{min}.{pat}")
    }

    pub fn compat_version_string(&self) -> String {
        let (maj, min, pat) = Self::decode_version(self.compatibility_version);
        format!("{maj}.{min}.{pat}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_version_10_6_0() {
        assert_eq!(DylibRef::decode_version(0x000A_0600), (10, 6, 0));
    }

    #[test]
    fn decode_version_zero() {
        assert_eq!(DylibRef::decode_version(0), (0, 0, 0));
    }

    #[test]
    fn decode_version_max() {
        assert_eq!(DylibRef::decode_version(0xFFFF_FFFF), (0xFFFF, 0xFF, 0xFF));
    }

    #[test]
    fn current_version_string_formats() {
        let d = DylibRef {
            name: String::new(),
            compatibility_version: 0,
            current_version: 0x000C_0301,
            timestamp: 0,
        };
        assert_eq!(d.current_version_string(), "12.3.1");
    }

    #[test]
    fn compat_version_string_formats() {
        let d = DylibRef {
            name: String::new(),
            compatibility_version: 0x0001_0000,
            current_version: 0,
            timestamp: 0,
        };
        assert_eq!(d.compat_version_string(), "1.0.0");
    }
}
