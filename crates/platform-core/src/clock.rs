//! Clock and time abstraction.

/// Wall-clock and monotonic time services.
pub trait Clock: Send + Sync {
    /// Wall-clock time as (seconds, microseconds) since Unix epoch.
    fn wall_time_usec(&self) -> (i64, i32);

    /// Monotonic time in nanoseconds (for mach_absolute_time).
    fn monotonic_ns(&self) -> u64;

    /// UTC offset in seconds for the local timezone.
    fn utc_offset_sec(&self) -> i64;

    /// IANA timezone name (e.g., "America/New_York").
    fn timezone_name(&self) -> String;

    /// Decompose a Unix timestamp into calendar components (local time).
    ///
    /// Returns (year-1900, month 0-11, day 1-31, hour, minute, second,
    /// day-of-year, weekday 0=Sun, gmtoff).
    fn localtime(&self, unix_sec: i64) -> CalendarComponents;

    /// Convert local calendar components back to a Unix timestamp.
    fn mktime(&self, components: &mut CalendarComponents) -> i64;

    /// Sleep the calling thread for the given number of fractional seconds.
    fn sleep_secs(&self, seconds: f64);
}

/// Calendar date/time components (equivalent to `struct tm`).
#[derive(Debug, Clone, Default)]
pub struct CalendarComponents {
    pub sec: i32,    // 0-60
    pub min: i32,    // 0-59
    pub hour: i32,   // 0-23
    pub mday: i32,   // 1-31
    pub mon: i32,    // 0-11
    pub year: i32,   // years since 1900
    pub wday: i32,   // 0=Sunday
    pub yday: i32,   // 0-365
    pub isdst: i32,  // daylight saving flag
    pub gmtoff: i64, // seconds east of UTC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_components_default_zero() {
        let c = CalendarComponents::default();
        assert_eq!(c.sec, 0);
        assert_eq!(c.min, 0);
        assert_eq!(c.hour, 0);
        assert_eq!(c.mday, 0);
        assert_eq!(c.mon, 0);
        assert_eq!(c.year, 0);
        assert_eq!(c.wday, 0);
        assert_eq!(c.yday, 0);
        assert_eq!(c.isdst, 0);
        assert_eq!(c.gmtoff, 0);
    }

    #[test]
    fn calendar_components_fields_writable() {
        let c = CalendarComponents {
            sec: 30,
            min: 45,
            hour: 14,
            mday: 29,
            mon: 2,
            year: 126,
            wday: 0,
            yday: 87,
            isdst: 1,
            gmtoff: -18000,
        };
        assert_eq!(c.sec, 30);
        assert_eq!(c.gmtoff, -18000);
    }

    #[test]
    fn calendar_components_clone() {
        let c = CalendarComponents {
            hour: 12,
            year: 126,
            ..Default::default()
        };
        let c2 = c.clone();
        assert_eq!(c2.hour, 12);
        assert_eq!(c2.year, 126);
    }

    #[test]
    fn calendar_components_debug() {
        let c = CalendarComponents::default();
        let dbg = format!("{c:?}");
        assert!(dbg.contains("CalendarComponents"));
        assert!(dbg.contains("sec"));
        assert!(dbg.contains("gmtoff"));
    }
}
