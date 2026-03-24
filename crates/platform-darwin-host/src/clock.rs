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
}
