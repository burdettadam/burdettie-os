//! Linux clock/time via gettimeofday, clock_gettime, localtime_r.

use platform_core::clock::{CalendarComponents, Clock};

pub struct LinuxClock;

impl Clock for LinuxClock {
    fn wall_time_usec(&self) -> (i64, i32) {
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        unsafe { libc::gettimeofday(&mut tv, std::ptr::null_mut()) };
        // libc uses i64 microseconds on 64-bit Linux but i32 on Apple hosts.
        #[allow(clippy::cast_possible_truncation, clippy::unnecessary_cast)]
        (tv.tv_sec, tv.tv_usec as i32)
    }

    fn monotonic_ns(&self) -> u64 {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
        #[allow(clippy::cast_sign_loss)]
        let result = ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64;
        result
    }

    fn utc_offset_sec(&self) -> i64 {
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        unsafe { libc::gettimeofday(&mut tv, std::ptr::null_mut()) };
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&tv.tv_sec, &mut tm) };
        // `tm_gmtoff` is a C long, so this also widens it on 32-bit Linux.
        #[allow(clippy::unnecessary_cast)]
        let gmtoff = tm.tm_gmtoff as i64;
        gmtoff
    }

    fn timezone_name(&self) -> String {
        // Try TZ env first
        if let Ok(tz) = std::env::var("TZ") {
            if !tz.is_empty() {
                return tz;
            }
        }

        // Try /etc/localtime symlink (Linux)
        if let Ok(target) = std::fs::read_link("/etc/localtime") {
            let path = target.to_string_lossy();
            // Linux typically uses /usr/share/zoneinfo/ prefix
            if let Some(zone) = path.strip_prefix("/usr/share/zoneinfo/") {
                return zone.to_string();
            }
        }

        // Try /etc/timezone file (Debian/Ubuntu)
        if let Ok(tz) = std::fs::read_to_string("/etc/timezone") {
            let trimmed = tz.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
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
        // `tm_gmtoff` is a C long, so this also widens it on 32-bit Linux.
        #[allow(clippy::unnecessary_cast)]
        let gmtoff = tm.tm_gmtoff as i64;
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
            gmtoff,
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
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let secs = seconds as u64;
            #[allow(clippy::cast_possible_truncation)]
            let nanos = ((seconds - secs as f64) * 1_000_000_000.0) as i64;
            let ts = libc::timespec {
                tv_sec: secs as libc::time_t,
                tv_nsec: nanos,
            };
            unsafe { libc::nanosleep(&ts, std::ptr::null_mut()) };
        }
    }
}
