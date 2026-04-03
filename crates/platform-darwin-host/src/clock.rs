//! macOS clock/time via gettimeofday, localtime_r, mach_absolute_time.

use platform_core::clock::{CalendarComponents, Clock};

pub struct DarwinClock;

impl Clock for DarwinClock {
    fn wall_time_usec(&self) -> (i64, i32) {
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        unsafe { libc::gettimeofday(&mut tv, std::ptr::null_mut()) };
        (tv.tv_sec, tv.tv_usec)
    }

    fn monotonic_ns(&self) -> u64 {
        #[allow(deprecated)]
        unsafe {
            libc::mach_absolute_time()
        }
    }

    fn utc_offset_sec(&self) -> i64 {
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        unsafe { libc::gettimeofday(&mut tv, std::ptr::null_mut()) };
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&tv.tv_sec, &mut tm) };
        tm.tm_gmtoff as i64
    }

    fn timezone_name(&self) -> String {
        // Try TZ env first
        if let Ok(tz) = std::env::var("TZ") {
            if !tz.is_empty() {
                return tz;
            }
        }

        // Try /etc/localtime symlink (macOS)
        let mut buf = [0u8; 256];
        let len = unsafe {
            libc::readlink(
                c"/etc/localtime".as_ptr(),
                buf.as_mut_ptr() as *mut i8,
                buf.len(),
            )
        };
        if len > 0 {
            let path = std::str::from_utf8(&buf[..len as usize]).unwrap_or("");
            if let Some(zone) = path.strip_prefix("/var/db/timezone/zoneinfo/") {
                return zone.to_string();
            }
        }

        let offset = self.utc_offset_sec();
        if offset == 0 {
            "UTC".to_string()
        } else {
            "GMT".to_string()
        }
    }

    fn localtime(&self, unix_sec: i64) -> CalendarComponents {
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&unix_sec, &mut tm) };
        CalendarComponents {
            sec: tm.tm_sec,
            min: tm.tm_min,
            hour: tm.tm_hour,
            mday: tm.tm_mday,
            mon: tm.tm_mon,
            year: tm.tm_year,
            wday: tm.tm_wday,
            yday: tm.tm_yday,
            isdst: tm.tm_isdst,
            gmtoff: tm.tm_gmtoff as i64,
        }
    }

    fn mktime(&self, components: &mut CalendarComponents) -> i64 {
        let mut tm = libc::tm {
            tm_sec: components.sec,
            tm_min: components.min,
            tm_hour: components.hour,
            tm_mday: components.mday,
            tm_mon: components.mon,
            tm_year: components.year,
            tm_wday: components.wday,
            tm_yday: components.yday,
            tm_isdst: components.isdst,
            tm_gmtoff: components.gmtoff as libc::c_long,
            tm_zone: std::ptr::null_mut(),
        };
        let result = unsafe { libc::mktime(&mut tm) };
        // Update components with normalized values
        components.sec = tm.tm_sec;
        components.min = tm.tm_min;
        components.hour = tm.tm_hour;
        components.mday = tm.tm_mday;
        components.mon = tm.tm_mon;
        components.year = tm.tm_year;
        components.wday = tm.tm_wday;
        components.yday = tm.tm_yday;
        result
    }

    fn sleep_secs(&self, seconds: f64) {
        if seconds > 0.0 {
            let secs = seconds as u64;
            let nanos = ((seconds - secs as f64) * 1_000_000_000.0) as i64;
            let ts = libc::timespec {
                tv_sec: secs as libc::time_t,
                tv_nsec: nanos,
            };
            unsafe { libc::nanosleep(&ts, std::ptr::null_mut()) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_time_after_2020() {
        let c = DarwinClock;
        let (sec, _usec) = c.wall_time_usec();
        // 2020-01-01 00:00:00 UTC = 1577836800
        assert!(sec > 1_577_836_800);
    }

    #[test]
    fn monotonic_nonzero() {
        let c = DarwinClock;
        assert!(c.monotonic_ns() > 0);
    }

    #[test]
    fn monotonic_is_monotonic() {
        let c = DarwinClock;
        let a = c.monotonic_ns();
        let b = c.monotonic_ns();
        assert!(b >= a);
    }

    #[test]
    fn timezone_name_nonempty() {
        let c = DarwinClock;
        let name = c.timezone_name();
        assert!(!name.is_empty());
    }

    #[test]
    fn localtime_epoch_zero() {
        let c = DarwinClock;
        let comps = c.localtime(0);
        // tm_year is years since 1900; epoch 0 is 1970 UTC but may
        // show as Dec 31, 1969 (year=69) in western timezones.
        assert!(comps.year == 69 || comps.year == 70);
    }

    #[test]
    fn mktime_roundtrip() {
        let c = DarwinClock;
        let ts: i64 = 1_700_000_000; // ~Nov 2023
        let mut comps = c.localtime(ts);
        let recovered = c.mktime(&mut comps);
        assert!((recovered - ts).abs() < 2);
    }

    #[test]
    fn utc_offset_in_range() {
        let c = DarwinClock;
        let off = c.utc_offset_sec();
        // ±14 hours = ±50400 seconds
        assert!(off >= -50400 && off <= 50400, "offset {off} out of range");
    }

    #[test]
    fn sleep_short_duration() {
        let c = DarwinClock;
        c.sleep_secs(0.001); // 1ms — should not crash
    }
}
