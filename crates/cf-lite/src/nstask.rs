//! NSTask shim: subprocess execution via posix_spawn.
//!
//! Object layout (48 bytes):
//!   offset  0: isa              (u64)
//!   offset  8: launch_path_ptr  (u64 — NSString)
//!   offset 16: arguments_ptr    (u64 — NSArray of NSString)
//!   offset 24: environment_ptr  (u64 — NSDictionary, 0=inherit)
//!   offset 32: stdout_pipe_ptr  (u64 — NSPipe/NSFileHandle, 0=inherit)
//!   offset 40: exit_status      (i32, valid after waitUntilExit)
//!   offset 44: pid              (i32)

use crate::cfstring;
use crate::nsobj;

const LAUNCH_PATH_OFF: usize = 8;
const ARGUMENTS_OFF: usize = 16;
const ENVIRONMENT_OFF: usize = 24;
const STDOUT_OFF: usize = 32;
const EXIT_STATUS_OFF: usize = 40;
const PID_OFF: usize = 44;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn alloc_task() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 48) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nstask_class_addr();
        *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = -1;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSTask alloc]`
pub extern "C" fn shim_nstask_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_task()
}

/// `-[NSTask init]`
pub extern "C" fn shim_nstask_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

// ── Setters ────────────────────────────────────────────────────

pub extern "C" fn shim_nstask_set_launch_path(obj: u64, _cmd: u64, path: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *mut u64) = path; }
    0
}

pub extern "C" fn shim_nstask_set_executable_url(obj: u64, _cmd: u64, url: u64) -> u64 {
    if obj == 0 || url == 0 { return 0; }
    let path_str = crate::nsurl::shim_nsurl_path(url, 0);
    unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *mut u64) = path_str; }
    0
}

pub extern "C" fn shim_nstask_set_arguments(obj: u64, _cmd: u64, args: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ARGUMENTS_OFF) as *mut u64) = args; }
    0
}

pub extern "C" fn shim_nstask_set_environment(obj: u64, _cmd: u64, env: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ENVIRONMENT_OFF) as *mut u64) = env; }
    0
}

pub extern "C" fn shim_nstask_set_stdout(obj: u64, _cmd: u64, pipe: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + STDOUT_OFF) as *mut u64) = pipe; }
    0
}

// ── Getters ────────────────────────────────────────────────────

pub extern "C" fn shim_nstask_launch_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *const u64) }
}

pub extern "C" fn shim_nstask_arguments(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ARGUMENTS_OFF) as *const u64) }
}

pub extern "C" fn shim_nstask_environment(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ENVIRONMENT_OFF) as *const u64) }
}

pub extern "C" fn shim_nstask_termination_status(obj: u64, _cmd: u64) -> i32 {
    if obj == 0 { return -1; }
    unsafe { *((obj as usize + EXIT_STATUS_OFF) as *const i32) }
}

pub extern "C" fn shim_nstask_process_identifier(obj: u64, _cmd: u64) -> i32 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + PID_OFF) as *const i32) }
}

pub extern "C" fn shim_nstask_is_running(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    if pid <= 0 { return 0; }
    let rc = unsafe { libc::kill(pid, 0) };
    u8::from(rc == 0)
}

/// `-[NSTask launch]` — spawn the subprocess.
pub extern "C" fn shim_nstask_launch(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let path_ptr = unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *const u64) };
    let Some(path) = cfstring::extract_cfstring_cstr(path_ptr) else { return 0; };
    let c_path = std::ffi::CString::new(path).unwrap_or_default();

    // Build argv
    let mut c_args: Vec<std::ffi::CString> = vec![c_path.clone()];
    let args_ptr = unsafe { *((obj as usize + ARGUMENTS_OFF) as *const u64) };
    if args_ptr != 0 {
        let n = crate::nsarray::shim_nsarray_count(args_ptr, 0);
        for i in 0..n {
            let arg = crate::nsarray::shim_nsarray_object_at_index(args_ptr, 0, i);
            if let Some(s) = cfstring::extract_cfstring_cstr(arg) {
                if let Ok(cs) = std::ffi::CString::new(s) {
                    c_args.push(cs);
                }
            }
        }
    }
    let mut argv_ptrs: Vec<*mut i8> = c_args.iter().map(|a| a.as_ptr() as *mut i8).collect();
    argv_ptrs.push(std::ptr::null_mut());

    let mut pid: libc::pid_t = 0;
    let rc = unsafe {
        libc::posix_spawn(
            &mut pid,
            c_path.as_ptr(),
            std::ptr::null(),   // file actions
            std::ptr::null(),   // spawn attrs
            argv_ptrs.as_ptr(),
            std::ptr::null(),   // inherit environment
        )
    };

    if rc == 0 {
        unsafe { *((obj as usize + PID_OFF) as *mut i32) = pid; }
    }
    0
}

/// `-[NSTask waitUntilExit]` — block until subprocess finishes.
pub extern "C" fn shim_nstask_wait_until_exit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    if pid <= 0 { return 0; }

    let mut status: i32 = 0;
    unsafe { libc::waitpid(pid, &mut status, 0); }

    let exit_code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        -1
    };
    unsafe { *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = exit_code; }
    0
}

/// `+[NSTask launchedTaskWithLaunchPath:arguments:]`
pub extern "C" fn shim_nstask_launched_with_path(
    _cls: u64, _cmd: u64, path: u64, args: u64,
) -> u64 {
    let obj = alloc_task();
    if obj == 0 { return 0; }
    unsafe {
        *((obj as usize + LAUNCH_PATH_OFF) as *mut u64) = path;
        *((obj as usize + ARGUMENTS_OFF) as *mut u64) = args;
    }
    shim_nstask_launch(obj, 0);
    obj
}

pub extern "C" fn shim_nstask_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let path = cfstring::extract_cfstring_cstr(
        unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *const u64) }
    ).unwrap_or("(null)");
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    make_nsstring(&format!("<NSTask: {path} pid={pid}>"))
}
