//! `launch_msg()` — the main IPC entry point for the launch API.
//!
//! In Apple's implementation, `launch_msg()` sends a message to launchd
//! over a Unix socket and receives a reply.  In our implementation,
//! everything runs in-process: the request is interpreted directly and
//! a reply `launch_data_t` is returned.
//!
//! Also provides `launch_get_fd()` which returns -1 (no real IPC socket).

use crate::constants::*;
use crate::job;
use crate::launch_data::{self, LaunchData, LaunchDataT, LaunchDataType, LAUNCH_DATA_NULL};

/// `launch_get_fd()` → fd for async I/O.
///
/// In our in-process implementation there is no real IPC socket,
/// so we return -1 (no fd available).
pub fn launch_get_fd() -> i32 {
    -1
}

/// `launch_msg(request)` → reply.
///
/// Routes the request based on its type:
/// - String: the string value is the command key (CheckIn, SubmitJob, etc.)
/// - Dictionary: first key determines the command
///
/// Returns a reply `launch_data_t` (caller owns it) or NULL on error.
pub fn launch_msg(request: LaunchDataT) -> LaunchDataT {
    if request.is_null() {
        return LAUNCH_DATA_NULL;
    }

    let ty = launch_data::get_type(request);

    // String request: the string itself is the command key
    if ty == LaunchDataType::String as u32 {
        let sptr = launch_data::get_string(request);
        if sptr.is_null() {
            return make_errno_reply(libc::EINVAL);
        }
        let cmd = unsafe { std::ffi::CStr::from_ptr(sptr) };
        let Ok(cmd_str) = cmd.to_str() else {
            return make_errno_reply(libc::EINVAL);
        };
        return handle_string_command(cmd_str, LAUNCH_DATA_NULL);
    }

    // Dictionary request: look for command key
    if ty == LaunchDataType::Dictionary as u32 {
        return handle_dict_request(request);
    }

    make_errno_reply(libc::EINVAL)
}

/// Handle a dictionary request.
///
/// The dictionary should have one top-level key which is the command
/// (e.g. "SubmitJob" → value is the job dictionary).
fn handle_dict_request(dict: LaunchDataT) -> LaunchDataT {
    // Try known command keys in order of likelihood

    // SubmitJob
    let val = lookup_key(dict, LAUNCH_KEY_SUBMITJOB);
    if !val.is_null() {
        return handle_submit_job(val);
    }

    // RemoveJob
    let val = lookup_key(dict, LAUNCH_KEY_REMOVEJOB);
    if !val.is_null() {
        return handle_remove_job(val);
    }

    // StartJob
    let val = lookup_key(dict, LAUNCH_KEY_STARTJOB);
    if !val.is_null() {
        return handle_start_job(val);
    }

    // StopJob
    let val = lookup_key(dict, LAUNCH_KEY_STOPJOB);
    if !val.is_null() {
        return handle_stop_job(val);
    }

    // GetJob
    let val = lookup_key(dict, LAUNCH_KEY_GETJOB);
    if !val.is_null() {
        return handle_get_job(val);
    }

    // GetJobs
    let val = lookup_key(dict, LAUNCH_KEY_GETJOBS);
    if !val.is_null() {
        return handle_get_jobs();
    }

    // CheckIn
    let val = lookup_key(dict, LAUNCH_KEY_CHECKIN);
    if !val.is_null() {
        return handle_checkin(val);
    }

    // SetUserEnvironment
    let val = lookup_key(dict, LAUNCH_KEY_SETUSERENVIRONMENT);
    if !val.is_null() {
        return handle_set_user_environment(val);
    }

    // UnsetUserEnvironment
    let val = lookup_key(dict, LAUNCH_KEY_UNSETUSERENVIRONMENT);
    if !val.is_null() {
        return handle_unset_user_environment(val);
    }

    log::debug!("launch_msg: unrecognized dictionary request");
    make_errno_reply(libc::ENOTSUP)
}

/// Handle a bare string command (no associated value).
fn handle_string_command(cmd: &str, _value: LaunchDataT) -> LaunchDataT {
    match cmd {
        LAUNCH_KEY_CHECKIN => {
            // CheckIn with no specific label — use env hint or return error
            let label = std::env::var("LAUNCHD_JOB_LABEL").ok();
            match label {
                Some(l) => match job::checkin(&l) {
                    Some(j) => j.to_launch_data(),
                    None => make_errno_reply(libc::ESRCH),
                },
                None => make_errno_reply(libc::ESRCH),
            }
        }
        LAUNCH_KEY_GETJOBS => handle_get_jobs(),
        LAUNCH_KEY_SHUTDOWN => {
            log::info!("launch_msg: shutdown requested");
            make_errno_reply(0) // success
        }
        LAUNCH_KEY_SINGLEUSER => {
            log::info!("launch_msg: single-user mode requested");
            make_errno_reply(0)
        }
        _ => {
            log::debug!("launch_msg: unrecognized string command '{}'", cmd);
            make_errno_reply(libc::ENOTSUP)
        }
    }
}

// ── Command handlers ────────────────────────────────────────────────

fn handle_submit_job(job_dict: LaunchDataT) -> LaunchDataT {
    match job::Job::from_launch_data(job_dict) {
        Some(j) => {
            let label = j.label.clone();
            match job::submit_job(j) {
                Ok(()) => {
                    log::info!("launch_msg: submitted job '{}'", label);
                    make_errno_reply(0)
                }
                Err(e) => {
                    log::warn!("launch_msg: submit failed for '{}': {}", label, e);
                    make_errno_reply(libc::EEXIST)
                }
            }
        }
        None => {
            log::warn!("launch_msg: SubmitJob with invalid job dictionary");
            make_errno_reply(libc::EINVAL)
        }
    }
}

fn handle_remove_job(value: LaunchDataT) -> LaunchDataT {
    let label = extract_string(value);
    match label {
        Some(l) => match job::remove_job(&l) {
            Ok(()) => make_errno_reply(0),
            Err(_) => make_errno_reply(libc::ESRCH),
        },
        None => make_errno_reply(libc::EINVAL),
    }
}

fn handle_start_job(value: LaunchDataT) -> LaunchDataT {
    let label = extract_string(value);
    match label {
        Some(l) => {
            if job::get_job(&l).is_some() {
                log::info!("launch_msg: start job '{}' (no-op in emulation)", l);
                make_errno_reply(0)
            } else {
                make_errno_reply(libc::ESRCH)
            }
        }
        None => make_errno_reply(libc::EINVAL),
    }
}

fn handle_stop_job(value: LaunchDataT) -> LaunchDataT {
    let label = extract_string(value);
    match label {
        Some(l) => {
            if job::get_job(&l).is_some() {
                log::info!("launch_msg: stop job '{}' (no-op in emulation)", l);
                make_errno_reply(0)
            } else {
                make_errno_reply(libc::ESRCH)
            }
        }
        None => make_errno_reply(libc::EINVAL),
    }
}

fn handle_get_job(value: LaunchDataT) -> LaunchDataT {
    let label = extract_string(value);
    match label {
        Some(l) => match job::get_job(&l) {
            Some(j) => j.to_launch_data(),
            None => make_errno_reply(libc::ESRCH),
        },
        None => make_errno_reply(libc::EINVAL),
    }
}

fn handle_get_jobs() -> LaunchDataT {
    let labels = job::get_all_job_labels();
    let dict = launch_data::alloc(LaunchDataType::Dictionary as u32);
    for label in &labels {
        if let Some(j) = job::get_job(label) {
            let pid = launch_data::new_integer(i64::from(j.pid));
            let c_key = std::ffi::CString::new(label.as_str()).unwrap_or_default();
            launch_data::dict_insert(dict, pid, c_key.as_ptr());
        }
    }
    dict
}

fn handle_checkin(value: LaunchDataT) -> LaunchDataT {
    // value may be a bool (true = check in current job) or a string (label)
    let ty = launch_data::get_type(value);
    let label = if ty == LaunchDataType::String as u32 {
        extract_string(value)
    } else {
        // Use env var as label hint
        std::env::var("LAUNCHD_JOB_LABEL").ok()
    };

    match label {
        Some(l) => match job::checkin(&l) {
            Some(j) => j.to_launch_data(),
            None => make_errno_reply(libc::ESRCH),
        },
        None => make_errno_reply(libc::ESRCH),
    }
}

fn handle_set_user_environment(value: LaunchDataT) -> LaunchDataT {
    // Expect a dictionary of key=value pairs
    if launch_data::get_type(value) != LaunchDataType::Dictionary as u32 {
        return make_errno_reply(libc::EINVAL);
    }

    unsafe extern "C" fn set_env(
        val: LaunchDataT,
        key: *const libc::c_char,
        _ctx: *mut libc::c_void,
    ) {
        if key.is_null() || val.is_null() {
            return;
        }
        let sptr = launch_data::get_string(val);
        if sptr.is_null() {
            return;
        }
        let k = unsafe { std::ffi::CStr::from_ptr(key) };
        let v = unsafe { std::ffi::CStr::from_ptr(sptr) };
        if let (Ok(ks), Ok(vs)) = (k.to_str(), v.to_str()) {
            std::env::set_var(ks, vs);
            log::debug!("launch_msg: setenv {}={}", ks, vs);
        }
    }

    launch_data::dict_iterate(value, set_env, std::ptr::null_mut());
    make_errno_reply(0)
}

fn handle_unset_user_environment(value: LaunchDataT) -> LaunchDataT {
    let name = extract_string(value);
    if let Some(n) = name {
        unsafe { std::env::remove_var(&n) };
        log::debug!("launch_msg: unsetenv {}", n);
        make_errno_reply(0)
    } else {
        make_errno_reply(libc::EINVAL)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn lookup_key(dict: LaunchDataT, key: &str) -> LaunchDataT {
    let c_key = std::ffi::CString::new(key).unwrap_or_default();
    launch_data::dict_lookup(dict, c_key.as_ptr())
}

fn extract_string(handle: LaunchDataT) -> Option<String> {
    if handle.is_null() {
        return None;
    }
    let ty = launch_data::get_type(handle);
    if ty == LaunchDataType::String as u32 {
        let ptr = launch_data::get_string(handle);
        if ptr.is_null() {
            return None;
        }
        let cs = unsafe { std::ffi::CStr::from_ptr(ptr) };
        cs.to_str().ok().map(|s| s.to_string())
    } else {
        None
    }
}

fn make_errno_reply(errno: i32) -> LaunchDataT {
    let d = launch_data::alloc(LaunchDataType::Errno as u32);
    if !d.is_null() {
        // Replace with the right errno
        if let Some(data) = unsafe { (d as *mut LaunchData).as_mut() } {
            *data = LaunchData::Errno(errno);
        }
    }
    d
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch_data::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Generate a unique job label to avoid inter-test interference.
    fn unique_label(prefix: &str) -> String {
        static CTR: AtomicU32 = AtomicU32::new(0);
        let n = CTR.fetch_add(1, Ordering::Relaxed);
        format!("com.lmsg.{}.{}", prefix, n)
    }

    #[test]
    fn launch_get_fd_returns_minus_one() {
        assert_eq!(launch_get_fd(), -1);
    }

    #[test]
    fn null_request_returns_null() {
        assert!(launch_msg(LAUNCH_DATA_NULL).is_null());
    }

    #[test]
    fn submit_and_get_via_launch_msg() {
        let label = unique_label("sub");
        let c_label = std::ffi::CString::new(label.clone()).unwrap();

        // Build a SubmitJob request
        let job_dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(job_dict, new_string(c_label.as_ptr()),
            c"Label".as_ptr());
        dict_insert(job_dict, new_string(c"/bin/true".as_ptr()),
            c"Program".as_ptr());

        let request = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(request, job_dict, c"SubmitJob".as_ptr());

        let reply = launch_msg(request);
        assert!(!reply.is_null());
        assert_eq!(get_type(reply), LaunchDataType::Errno as u32);
        assert_eq!(get_errno(reply), 0); // success
        free(reply);
        free(request);

        // GetJob
        let get_req = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(get_req, new_string(c_label.as_ptr()),
            c"GetJob".as_ptr());
        let reply = launch_msg(get_req);
        assert!(!reply.is_null());
        assert_eq!(get_type(reply), LaunchDataType::Dictionary as u32);
        assert_eq!(dict_get_string_value(reply, "Label").as_deref(), Some(label.as_str()));
        free(reply);
        free(get_req);
    }

    #[test]
    fn remove_via_launch_msg() {
        let label = unique_label("rm");
        let c_label = std::ffi::CString::new(label.clone()).unwrap();
        job::submit_job(job::Job::new(&label)).unwrap();

        let request = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(request, new_string(c_label.as_ptr()),
            c"RemoveJob".as_ptr());

        let reply = launch_msg(request);
        assert_eq!(get_errno(reply), 0);
        free(reply);
        free(request);

        assert!(job::get_job(&label).is_none());
    }

    #[test]
    fn get_jobs_returns_dict() {
        let l1 = unique_label("gj1");
        let l2 = unique_label("gj2");
        job::submit_job(job::Job::new(&l1)).unwrap();
        job::submit_job(job::Job::new(&l2)).unwrap();

        let request = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(request, new_bool(true), c"GetJobs".as_ptr());

        let reply = launch_msg(request);
        assert_eq!(get_type(reply), LaunchDataType::Dictionary as u32);
        // At least 2 jobs (may be more from parallel tests)
        assert!(dict_get_count(reply) >= 2);
        free(reply);
        free(request);
    }

    #[test]
    fn string_command_getjobs() {
        let label = unique_label("strj");
        job::submit_job(job::Job::new(&label)).unwrap();

        let request = new_string(c"GetJobs".as_ptr());
        let reply = launch_msg(request);
        assert_eq!(get_type(reply), LaunchDataType::Dictionary as u32);
        // At least 1 job
        assert!(dict_get_count(reply) >= 1);
        free(reply);
        free(request);
    }

    #[test]
    fn duplicate_submit_returns_eexist() {
        let label = unique_label("dup");
        let c_label = std::ffi::CString::new(label.clone()).unwrap();
        job::submit_job(job::Job::new(&label)).unwrap();

        let job_dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(job_dict, new_string(c_label.as_ptr()), c"Label".as_ptr());

        let request = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(request, job_dict, c"SubmitJob".as_ptr());

        let reply = launch_msg(request);
        assert_eq!(get_errno(reply), libc::EEXIST);
        free(reply);
        free(request);
    }
}
