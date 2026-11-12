//! NSTask shim: subprocess execution via posix_spawn.
//!
//! Object layout (72 bytes):
//!   offset  0: isa              (u64)
//!   offset  8: launch_path_ptr  (u64 — NSString)
//!   offset 16: arguments_ptr    (u64 — NSArray of NSString)
//!   offset 24: environment_ptr  (u64 — NSDictionary, 0=inherit)
//!   offset 32: stdout_pipe_ptr  (u64 — NSPipe/NSFileHandle, 0=inherit)
//!   offset 40: exit_status      (i32, valid after waitUntilExit)
//!   offset 44: pid              (i32)
//!   offset 48: stdin_pipe_ptr   (u64 — NSPipe/NSFileHandle for stdin, 0=inherit)
//!   offset 56: stderr_pipe_ptr  (u64 — NSPipe/NSFileHandle for stderr, 0=inherit)
//!   offset 64: current_dir_ptr  (u64 — NSString, 0=inherit parent cwd)

use crate::cfstring;
use crate::nsobj;
#[cfg(not(unix))]
use std::{
    collections::HashMap,
    process::{Child, Command, ExitStatus},
    sync::{Mutex, OnceLock},
};

const LAUNCH_PATH_OFF: usize = 8;
const ARGUMENTS_OFF: usize = 16;
const ENVIRONMENT_OFF: usize = 24;
const STDOUT_OFF: usize = 32;
const CURRENT_DIR_OFF: usize = 64;
const EXIT_STATUS_OFF: usize = 40;
const PID_OFF: usize = 44;
const STDIN_OFF: usize = 48;
const STDERR_OFF: usize = 56;

#[cfg(not(unix))]
fn child_registry() -> &'static Mutex<HashMap<i32, Child>> {
    static REGISTRY: OnceLock<Mutex<HashMap<i32, Child>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(not(unix))]
fn exit_code_from_status(status: ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn alloc_task() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 72) as u64;
    if obj == 0 {
        return 0;
    }
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
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + LAUNCH_PATH_OFF) as *mut u64) = path;
    }
    0
}

pub extern "C" fn shim_nstask_set_executable_url(obj: u64, _cmd: u64, url: u64) -> u64 {
    if obj == 0 || url == 0 {
        return 0;
    }
    let path_str = crate::nsurl::shim_nsurl_path(url, 0);
    unsafe {
        *((obj as usize + LAUNCH_PATH_OFF) as *mut u64) = path_str;
    }
    0
}

pub extern "C" fn shim_nstask_set_arguments(obj: u64, _cmd: u64, args: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + ARGUMENTS_OFF) as *mut u64) = args;
    }
    0
}

pub extern "C" fn shim_nstask_set_environment(obj: u64, _cmd: u64, env: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + ENVIRONMENT_OFF) as *mut u64) = env;
    }
    0
}

pub extern "C" fn shim_nstask_set_stdout(obj: u64, _cmd: u64, pipe: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + STDOUT_OFF) as *mut u64) = pipe;
    }
    0
}

/// `-[NSTask setStandardInput:]`
pub extern "C" fn shim_nstask_set_stdin(obj: u64, _cmd: u64, pipe: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + STDIN_OFF) as *mut u64) = pipe;
    }
    0
}

/// `-[NSTask setStandardError:]`
pub extern "C" fn shim_nstask_set_stderr(obj: u64, _cmd: u64, pipe: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + STDERR_OFF) as *mut u64) = pipe;
    }
    0
}

/// `-[NSTask standardOutput]`
pub extern "C" fn shim_nstask_get_stdout(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + STDOUT_OFF) as *const u64) }
}

/// `-[NSTask standardInput]`
pub extern "C" fn shim_nstask_get_stdin(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + STDIN_OFF) as *const u64) }
}

/// `-[NSTask standardError]`
pub extern "C" fn shim_nstask_get_stderr(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + STDERR_OFF) as *const u64) }
}

/// `-[NSTask setCurrentDirectoryPath:]`
pub extern "C" fn shim_nstask_set_current_dir(obj: u64, _cmd: u64, path: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + CURRENT_DIR_OFF) as *mut u64) = path;
    }
    0
}

/// `-[NSTask setCurrentDirectoryURL:]` — convenience wrapper taking an NSURL.
pub extern "C" fn shim_nstask_set_current_dir_url(obj: u64, _cmd: u64, url: u64) -> u64 {
    if obj == 0 || url == 0 {
        return 0;
    }
    let path = crate::nsurl::shim_nsurl_path(url, 0);
    unsafe {
        *((obj as usize + CURRENT_DIR_OFF) as *mut u64) = path;
    }
    0
}

/// `-[NSTask currentDirectoryPath]`
pub extern "C" fn shim_nstask_current_dir(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CURRENT_DIR_OFF) as *const u64) }
}

/// `-[NSTask currentDirectoryURL]` — return an NSURL for the stored current-directory path.
pub extern "C" fn shim_nstask_current_directory_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path = unsafe { *((obj as usize + CURRENT_DIR_OFF) as *const u64) };
    if path == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_file_url_with_path(0, 0, path)
}

/// `-[NSTask executableURL]` — return an NSURL for the stored launch-path.
pub extern "C" fn shim_nstask_executable_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path = unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *const u64) };
    if path == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_file_url_with_path(0, 0, path)
}

/// `NSTaskTerminationReason` — NSTaskTerminationReasonExit = 1
pub extern "C" fn shim_nstask_termination_reason(_obj: u64, _cmd: u64) -> i64 {
    1 // NSTaskTerminationReasonExit
}

// ── Getters ────────────────────────────────────────────────────

pub extern "C" fn shim_nstask_launch_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *const u64) }
}

pub extern "C" fn shim_nstask_arguments(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + ARGUMENTS_OFF) as *const u64) }
}

pub extern "C" fn shim_nstask_environment(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + ENVIRONMENT_OFF) as *const u64) }
}

pub extern "C" fn shim_nstask_termination_status(obj: u64, _cmd: u64) -> i32 {
    if obj == 0 {
        return -1;
    }
    unsafe { *((obj as usize + EXIT_STATUS_OFF) as *const i32) }
}

pub extern "C" fn shim_nstask_process_identifier(obj: u64, _cmd: u64) -> i32 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + PID_OFF) as *const i32) }
}

#[cfg(unix)]
pub extern "C" fn shim_nstask_is_running(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    if pid <= 0 {
        return 0;
    }
    let rc = unsafe { libc::kill(pid, 0) };
    u8::from(rc == 0)
}

#[cfg(not(unix))]
pub extern "C" fn shim_nstask_is_running(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    if pid <= 0 {
        return 0;
    }

    let mut registry = child_registry().lock().unwrap_or_else(|e| e.into_inner());
    let status = match registry
        .get_mut(&pid)
        .and_then(|child| child.try_wait().ok())
    {
        None => return 0,
        Some(None) => return 1,
        Some(Some(status)) => status,
    };
    registry.remove(&pid);
    unsafe {
        *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = exit_code_from_status(status);
        *((obj as usize + PID_OFF) as *mut i32) = 0;
    }
    0
}

/// Close and invalidate the child-facing end of a pipe in the parent after posix_spawn.
///
/// After the child has been spawned, `posix_spawn_file_actions_adddup2` has already
/// copied the relevant fd into the child.  The parent no longer needs that copy, and
/// keeping it open prevents `readDataToEndOfFile` from ever seeing EOF.
///
/// `is_stdin = false` → close the **write** end (child writes to it as stdout/stderr).
/// `is_stdin = true`  → close the **read** end (child reads from it as stdin).
///
/// Only acts on NSPipe objects; raw NSFileHandle objects are left alone (caller manages).
fn close_parent_pipe_end(obj: u64, is_stdin: bool) {
    if obj == 0 {
        return;
    }
    let isa = unsafe { *(obj as *const u64) };
    if isa != nsobj::nspipe_class_addr() {
        return;
    }
    // NSPipe layout: [isa:0, read_fh:8, write_fh:16]
    let fh_off: usize = if is_stdin { 8 } else { 16 };
    let fh = unsafe { *((obj as usize + fh_off) as *const u64) };
    if fh == 0 {
        return;
    }
    // NSFileHandle layout: [isa:0, fd:8, flags:16]
    let fd = unsafe { *((fh as usize + 8) as *const i64) } as i32;
    if fd >= 0 {
        unsafe {
            libc::close(fd);
        }
        unsafe {
            *((fh as usize + 8) as *mut i64) = -1;
        }
    }
}

/// Helper: get the file descriptor from an NSPipe or NSFileHandle object.
/// NSPipe layout: [isa, read_fh, write_fh]; NSFileHandle layout: [isa, fd, flags]
fn pipe_or_fh_to_fd(obj: u64) -> Option<i32> {
    if obj == 0 {
        return None;
    }
    let isa = unsafe { *(obj as *const u64) };
    if isa == nsobj::nspipe_class_addr() {
        // For NSPipe used as stdin: writing end for stdin, reading end for stdout/stderr
        // Caller must decide which end — we return the write end (offset 16) for stdin
        // and read end (offset 8) for captures. Since this is confusing, we track by offset:
        // When used as setStandardOutput/setStandardError, the pipe's *write* end is what
        // the child writes to; parent reads from the read end.
        // We return the write-end fd here; launch() will wire it to the child's stdout/stderr.
        let write_fh = unsafe { *((obj as usize + 16) as *const u64) };
        if write_fh == 0 {
            return None;
        }
        let fd = unsafe { *((write_fh as usize + 8) as *const i64) } as i32;
        Some(fd)
    } else if isa == nsobj::nsfilehandle_class_addr() {
        let fd = unsafe { *((obj as usize + 8) as *const i64) } as i32;
        Some(fd)
    } else {
        None
    }
}

/// Helper for stdin pipe: return the READ end fd of an NSPipe.
fn pipe_read_fd(obj: u64) -> Option<i32> {
    if obj == 0 {
        return None;
    }
    let isa = unsafe { *(obj as *const u64) };
    if isa == nsobj::nspipe_class_addr() {
        let read_fh = unsafe { *((obj as usize + 8) as *const u64) };
        if read_fh == 0 {
            return None;
        }
        let fd = unsafe { *((read_fh as usize + 8) as *const i64) } as i32;
        Some(fd)
    } else {
        pipe_or_fh_to_fd(obj)
    }
}

/// Build a null-terminated `*mut *mut i8` envp from an NSDictionary NSString→NSString.
/// Returns the CString owners (kept alive) and the pointer array.
fn build_envp(env_ptr: u64) -> (Vec<std::ffi::CString>, Vec<*mut std::ffi::c_char>) {
    let mut owners: Vec<std::ffi::CString> = Vec::new();
    let count = crate::nsdictionary::helpers_pub::dict_count_pub(env_ptr) as usize;
    let keys_ptr = crate::nsdictionary::helpers_pub::dict_keys_pub(env_ptr);
    let vals_ptr = crate::nsdictionary::helpers_pub::dict_vals_pub(env_ptr);
    for i in 0..count {
        let k = unsafe { *((keys_ptr as *const u64).add(i)) };
        let v = unsafe { *((vals_ptr as *const u64).add(i)) };
        if let (Some(ks), Some(vs)) = (
            cfstring::extract_cfstring_cstr(k),
            cfstring::extract_cfstring_cstr(v),
        ) {
            if let Ok(cs) = std::ffi::CString::new(format!("{ks}={vs}")) {
                owners.push(cs);
            }
        }
    }
    let mut ptrs: Vec<*mut std::ffi::c_char> = owners
        .iter()
        .map(|c| c.as_ptr() as *mut std::ffi::c_char)
        .collect();
    ptrs.push(std::ptr::null_mut());
    (owners, ptrs)
}

fn inherited_envp(
    env_ptr: u64,
    env_ptrs: &[*mut std::ffi::c_char],
) -> *const *mut std::ffi::c_char {
    if env_ptr != 0 {
        return env_ptrs.as_ptr();
    }

    unsafe {
        unsafe extern "C" {
            static mut environ: *mut *mut std::ffi::c_char;
        }

        environ as *const *mut std::ffi::c_char
    }
}

/// `-[NSTask launch]` — spawn the subprocess.
#[cfg(unix)]
pub extern "C" fn shim_nstask_launch(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path_ptr = unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *const u64) };
    let Some(path) = cfstring::extract_cfstring_cstr(path_ptr) else {
        return 0;
    };
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
    let mut argv_ptrs: Vec<*mut std::ffi::c_char> = c_args
        .iter()
        .map(|a| a.as_ptr() as *mut std::ffi::c_char)
        .collect();
    argv_ptrs.push(std::ptr::null_mut());

    // Build envp from NSDictionary if provided, else inherit
    let env_ptr = unsafe { *((obj as usize + ENVIRONMENT_OFF) as *const u64) };
    let (_env_owners, env_ptrs) = if env_ptr != 0 {
        let (owners, ptrs) = build_envp(env_ptr);
        (owners, ptrs)
    } else {
        (Vec::new(), Vec::new())
    };

    // Set up file actions for stdin/stdout/stderr pipes and optional chdir
    let stdout_obj = unsafe { *((obj as usize + STDOUT_OFF) as *const u64) };
    let stdin_obj = unsafe { *((obj as usize + STDIN_OFF) as *const u64) };
    let stderr_obj = unsafe { *((obj as usize + STDERR_OFF) as *const u64) };
    let current_dir_ptr = unsafe { *((obj as usize + CURRENT_DIR_OFF) as *const u64) };
    let c_dir = if current_dir_ptr != 0 {
        cfstring::extract_cfstring_cstr(current_dir_ptr)
            .and_then(|s| std::ffi::CString::new(s).ok())
    } else {
        None
    };

    let mut file_actions: libc::posix_spawn_file_actions_t = unsafe { std::mem::zeroed() };
    let has_actions = stdout_obj != 0 || stdin_obj != 0 || stderr_obj != 0 || c_dir.is_some();
    if has_actions {
        unsafe {
            libc::posix_spawn_file_actions_init(&mut file_actions);
        }
        // stdout: dup write-end of pipe to child's stdout (fd 1)
        if let Some(fd) = pipe_or_fh_to_fd(stdout_obj) {
            unsafe {
                libc::posix_spawn_file_actions_adddup2(&mut file_actions, fd, 1);
            }
        }
        // stderr: dup write-end of pipe to child's stderr (fd 2)
        if let Some(fd) = pipe_or_fh_to_fd(stderr_obj) {
            unsafe {
                libc::posix_spawn_file_actions_adddup2(&mut file_actions, fd, 2);
            }
        }
        // stdin: dup read-end of pipe to child's stdin (fd 0)
        if let Some(fd) = pipe_read_fd(stdin_obj) {
            unsafe {
                libc::posix_spawn_file_actions_adddup2(&mut file_actions, fd, 0);
            }
        }
        // chdir: change to currentDirectoryPath before exec
        if let Some(ref dir) = c_dir {
            // posix_spawn_file_actions_addchdir_np is available on macOS 10.15+.
            // Declare it directly since libc 0.2 may not expose it yet.
            extern "C" {
                fn posix_spawn_file_actions_addchdir_np(
                    file_actions: *mut libc::posix_spawn_file_actions_t,
                    path: *const libc::c_char,
                ) -> libc::c_int;
            }
            unsafe {
                posix_spawn_file_actions_addchdir_np(&mut file_actions, dir.as_ptr());
            }
        }
    }

    let mut pid: libc::pid_t = 0;
    let envp = inherited_envp(env_ptr, &env_ptrs);

    let rc = unsafe {
        libc::posix_spawn(
            &mut pid,
            c_path.as_ptr(),
            if has_actions {
                &file_actions
            } else {
                std::ptr::null()
            },
            std::ptr::null(), // spawn attrs
            argv_ptrs.as_ptr(),
            envp,
        )
    };

    if has_actions {
        unsafe {
            libc::posix_spawn_file_actions_destroy(&mut file_actions);
        }
    }

    if rc == 0 {
        unsafe {
            *((obj as usize + PID_OFF) as *mut i32) = pid;
        }
        // Close the parent's copy of each child-facing pipe end.  The child has already
        // inherited these via posix_spawn_file_actions dup2; leaving them open in the
        // parent would prevent readDataToEndOfFile from ever detecting EOF.
        close_parent_pipe_end(stdout_obj, false); // close write end — child writes stdout
        close_parent_pipe_end(stderr_obj, false); // close write end — child writes stderr
        close_parent_pipe_end(stdin_obj, true); // close read  end — child reads stdin
    } else {
        log::warn!(
            "NSTask launch failed: posix_spawn returned {rc} for {:?}",
            path_ptr
        );
    }
    0
}

#[cfg(not(unix))]
pub extern "C" fn shim_nstask_launch(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path_ptr = unsafe { *((obj as usize + LAUNCH_PATH_OFF) as *const u64) };
    let Some(path) = cfstring::extract_cfstring_cstr(path_ptr) else {
        return 0;
    };

    let mut command = Command::new(path);

    let args_ptr = unsafe { *((obj as usize + ARGUMENTS_OFF) as *const u64) };
    if args_ptr != 0 {
        let n = crate::nsarray::shim_nsarray_count(args_ptr, 0);
        for i in 0..n {
            let arg = crate::nsarray::shim_nsarray_object_at_index(args_ptr, 0, i);
            if let Some(s) = cfstring::extract_cfstring_cstr(arg) {
                command.arg(s);
            }
        }
    }

    let env_ptr = unsafe { *((obj as usize + ENVIRONMENT_OFF) as *const u64) };
    if env_ptr != 0 {
        let count = crate::nsdictionary::helpers_pub::dict_count_pub(env_ptr) as usize;
        let keys_ptr = crate::nsdictionary::helpers_pub::dict_keys_pub(env_ptr);
        let vals_ptr = crate::nsdictionary::helpers_pub::dict_vals_pub(env_ptr);
        for i in 0..count {
            let k = unsafe { *((keys_ptr as *const u64).add(i)) };
            let v = unsafe { *((vals_ptr as *const u64).add(i)) };
            if let (Some(ks), Some(vs)) = (
                cfstring::extract_cfstring_cstr(k),
                cfstring::extract_cfstring_cstr(v),
            ) {
                command.env(ks, vs);
            }
        }
    }

    let current_dir_ptr = unsafe { *((obj as usize + CURRENT_DIR_OFF) as *const u64) };
    if current_dir_ptr != 0 {
        if let Some(dir) = cfstring::extract_cfstring_cstr(current_dir_ptr) {
            command.current_dir(dir);
        }
    }

    if unsafe { *((obj as usize + STDOUT_OFF) as *const u64) } != 0
        || unsafe { *((obj as usize + STDERR_OFF) as *const u64) } != 0
        || unsafe { *((obj as usize + STDIN_OFF) as *const u64) } != 0
    {
        log::warn!("NSTask: pipe redirection is not implemented for this host backend");
    }

    match command.spawn() {
        Ok(child) => {
            let pid = child.id() as i32;
            unsafe {
                *((obj as usize + PID_OFF) as *mut i32) = pid;
            }
            child_registry()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(pid, child);
        }
        Err(err) => {
            log::warn!("NSTask launch failed for {path:?}: {err}");
        }
    }
    0
}

/// `-[NSTask launchAndReturnError:]`
pub extern "C" fn shim_nstask_launch_and_return_error(obj: u64, _cmd: u64, error_out: u64) -> u8 {
    let result = shim_nstask_launch(obj, 0);
    let _ = result;
    // Check if pid was set
    if obj != 0 {
        let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
        if pid > 0 {
            if error_out != 0 {
                unsafe {
                    *(error_out as *mut u64) = 0;
                }
            }
            return 1; // YES
        }
    }
    if error_out != 0 {
        let err = crate::nserror::make_nserror("NSPOSIXErrorDomain", 2); // ENOENT
        unsafe {
            *(error_out as *mut u64) = err;
        }
    }
    0 // NO
}

/// `-[NSTask waitUntilExit]` — block until subprocess finishes.
#[cfg(unix)]
pub extern "C" fn shim_nstask_wait_until_exit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    if pid <= 0 {
        return 0;
    }

    let mut status: i32 = 0;
    unsafe {
        libc::waitpid(pid, &mut status, 0);
    }

    let exit_code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        -1
    };
    unsafe {
        *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = exit_code;
    }
    0
}

#[cfg(not(unix))]
pub extern "C" fn shim_nstask_wait_until_exit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    if pid <= 0 {
        return 0;
    }

    let child = child_registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&pid);
    let Some(mut child) = child else {
        return 0;
    };
    let exit_code = child.wait().map(exit_code_from_status).unwrap_or(-1);
    unsafe {
        *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = exit_code;
        *((obj as usize + PID_OFF) as *mut i32) = 0;
    }
    0
}

/// `+[NSTask launchedTaskWithLaunchPath:arguments:]`
pub extern "C" fn shim_nstask_launched_with_path(
    _cls: u64,
    _cmd: u64,
    path: u64,
    args: u64,
) -> u64 {
    let obj = alloc_task();
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + LAUNCH_PATH_OFF) as *mut u64) = path;
        *((obj as usize + ARGUMENTS_OFF) as *mut u64) = args;
    }
    shim_nstask_launch(obj, 0);
    obj
}

pub extern "C" fn shim_nstask_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path = cfstring::extract_cfstring_cstr(unsafe {
        *((obj as usize + LAUNCH_PATH_OFF) as *const u64)
    })
    .unwrap_or("(null)");
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    make_nsstring(&format!("<NSTask: {path} pid={pid}>"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── alloc / init ───────────────────────────────────────────

    #[test]
    fn test_alloc_task_helper() {
        let obj = alloc_task();
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), crate::nsobj::nstask_class_addr());
        // exit_status initialised to -1
        let exit = unsafe { *((obj as usize + EXIT_STATUS_OFF) as *const i32) };
        assert_eq!(exit, -1);
        free_obj(obj);
    }

    #[test]
    fn test_shim_alloc() {
        let obj = shim_nstask_alloc(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), crate::nsobj::nstask_class_addr());
        free_obj(obj);
    }

    #[test]
    fn test_init_returns_self() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_init(obj, 0), obj);
        free_obj(obj);
    }

    #[test]
    fn test_init_nil() {
        assert_eq!(shim_nstask_init(0, 0), 0);
    }

    // ── launch_path setter / getter ────────────────────────────

    #[test]
    fn test_set_get_launch_path() {
        let obj = alloc_task();
        let path = make_string("/usr/bin/echo");
        shim_nstask_set_launch_path(obj, 0, path);
        assert_eq!(shim_nstask_launch_path(obj, 0), path);
        free_obj(obj);
        free_obj(path);
    }

    #[test]
    fn test_set_launch_path_nil() {
        assert_eq!(shim_nstask_set_launch_path(0, 0, 42), 0);
    }

    #[test]
    fn test_launch_path_nil() {
        assert_eq!(shim_nstask_launch_path(0, 0), 0);
    }

    // ── arguments setter / getter ──────────────────────────────

    #[test]
    fn test_set_get_arguments() {
        let obj = alloc_task();
        let args: u64 = 0xBEEF;
        shim_nstask_set_arguments(obj, 0, args);
        assert_eq!(shim_nstask_arguments(obj, 0), args);
        free_obj(obj);
    }

    #[test]
    fn test_set_arguments_nil() {
        assert_eq!(shim_nstask_set_arguments(0, 0, 42), 0);
    }

    #[test]
    fn test_arguments_nil() {
        assert_eq!(shim_nstask_arguments(0, 0), 0);
    }

    // ── environment setter / getter ────────────────────────────

    #[test]
    fn test_set_get_environment() {
        let obj = alloc_task();
        let env: u64 = 0xCAFE;
        shim_nstask_set_environment(obj, 0, env);
        assert_eq!(shim_nstask_environment(obj, 0), env);
        free_obj(obj);
    }

    #[test]
    fn test_set_environment_nil() {
        assert_eq!(shim_nstask_set_environment(0, 0, 42), 0);
    }

    #[test]
    fn test_environment_nil() {
        assert_eq!(shim_nstask_environment(0, 0), 0);
    }

    // ── stdout setter / getter ─────────────────────────────────

    #[test]
    fn test_set_get_stdout() {
        let obj = alloc_task();
        let pipe: u64 = 0x1234;
        shim_nstask_set_stdout(obj, 0, pipe);
        assert_eq!(shim_nstask_get_stdout(obj, 0), pipe);
        free_obj(obj);
    }

    #[test]
    fn test_set_stdout_nil() {
        assert_eq!(shim_nstask_set_stdout(0, 0, 42), 0);
    }

    #[test]
    fn test_get_stdout_nil() {
        assert_eq!(shim_nstask_get_stdout(0, 0), 0);
    }

    // ── stdin setter / getter ──────────────────────────────────

    #[test]
    fn test_set_get_stdin() {
        let obj = alloc_task();
        let pipe: u64 = 0x5678;
        shim_nstask_set_stdin(obj, 0, pipe);
        assert_eq!(shim_nstask_get_stdin(obj, 0), pipe);
        free_obj(obj);
    }

    #[test]
    fn test_set_stdin_nil() {
        assert_eq!(shim_nstask_set_stdin(0, 0, 42), 0);
    }

    #[test]
    fn test_get_stdin_nil() {
        assert_eq!(shim_nstask_get_stdin(0, 0), 0);
    }

    // ── stderr setter / getter ─────────────────────────────────

    #[test]
    fn test_set_get_stderr() {
        let obj = alloc_task();
        let pipe: u64 = 0x9ABC;
        shim_nstask_set_stderr(obj, 0, pipe);
        assert_eq!(shim_nstask_get_stderr(obj, 0), pipe);
        free_obj(obj);
    }

    #[test]
    fn test_set_stderr_nil() {
        assert_eq!(shim_nstask_set_stderr(0, 0, 42), 0);
    }

    #[test]
    fn test_get_stderr_nil() {
        assert_eq!(shim_nstask_get_stderr(0, 0), 0);
    }

    // ── currentDirectoryPath setter / getter ───────────────────

    #[test]
    fn test_set_get_current_dir() {
        let obj = alloc_task();
        let dir = make_string("/tmp");
        shim_nstask_set_current_dir(obj, 0, dir);
        assert_eq!(shim_nstask_current_dir(obj, 0), dir);
        free_obj(obj);
        free_obj(dir);
    }

    #[test]
    fn test_set_current_dir_nil() {
        assert_eq!(shim_nstask_set_current_dir(0, 0, 42), 0);
    }

    #[test]
    fn test_current_dir_nil() {
        assert_eq!(shim_nstask_current_dir(0, 0), 0);
    }

    // ── setExecutableURL / setCurrentDirectoryURL (nil safety) ─

    #[test]
    fn test_set_executable_url_nil_obj() {
        assert_eq!(shim_nstask_set_executable_url(0, 0, 42), 0);
    }

    #[test]
    fn test_set_executable_url_nil_url() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_set_executable_url(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_set_current_dir_url_nil_obj() {
        assert_eq!(shim_nstask_set_current_dir_url(0, 0, 42), 0);
    }

    #[test]
    fn test_set_current_dir_url_nil_url() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_set_current_dir_url(obj, 0, 0), 0);
        free_obj(obj);
    }

    // ── terminationReason ──────────────────────────────────────

    #[test]
    fn test_termination_reason_always_one() {
        assert_eq!(shim_nstask_termination_reason(0, 0), 1);
        let obj = alloc_task();
        assert_eq!(shim_nstask_termination_reason(obj, 0), 1);
        free_obj(obj);
    }

    // ── terminationStatus ──────────────────────────────────────

    #[test]
    fn test_termination_status_default() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_termination_status(obj, 0), -1);
        free_obj(obj);
    }

    #[test]
    fn test_termination_status_nil() {
        assert_eq!(shim_nstask_termination_status(0, 0), -1);
    }

    #[test]
    fn test_termination_status_after_write() {
        let obj = alloc_task();
        unsafe {
            *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = 0;
        }
        assert_eq!(shim_nstask_termination_status(obj, 0), 0);
        unsafe {
            *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = 42;
        }
        assert_eq!(shim_nstask_termination_status(obj, 0), 42);
        free_obj(obj);
    }

    // ── processIdentifier ──────────────────────────────────────

    #[test]
    fn test_process_identifier_default() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_process_identifier(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_process_identifier_nil() {
        assert_eq!(shim_nstask_process_identifier(0, 0), 0);
    }

    #[test]
    fn test_process_identifier_after_write() {
        let obj = alloc_task();
        unsafe {
            *((obj as usize + PID_OFF) as *mut i32) = 12345;
        }
        assert_eq!(shim_nstask_process_identifier(obj, 0), 12345);
        free_obj(obj);
    }

    // ── isRunning ──────────────────────────────────────────────

    #[test]
    fn test_is_running_nil() {
        assert_eq!(shim_nstask_is_running(0, 0), 0);
    }

    #[test]
    fn test_is_running_no_pid() {
        let obj = alloc_task();
        // pid is 0 from calloc → returns 0 without calling kill
        assert_eq!(shim_nstask_is_running(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_is_running_negative_pid() {
        let obj = alloc_task();
        unsafe {
            *((obj as usize + PID_OFF) as *mut i32) = -1;
        }
        assert_eq!(shim_nstask_is_running(obj, 0), 0);
        free_obj(obj);
    }

    // ── currentDirectoryURL ────────────────────────────────────

    #[test]
    fn test_current_directory_url_nil() {
        assert_eq!(shim_nstask_current_directory_url(0, 0), 0);
    }

    #[test]
    fn test_current_directory_url_no_path() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_current_directory_url(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_current_directory_url_returns_nsurl() {
        let obj = alloc_task();
        let dir = make_string("/tmp");
        shim_nstask_set_current_dir(obj, 0, dir);
        let url = shim_nstask_current_directory_url(obj, 0);
        assert_ne!(url, 0);
        // Extract path from the NSURL → should match original
        let path = crate::nsurl::shim_nsurl_path(url, 0);
        assert_string_eq(path, "/tmp");
        free_obj(obj);
    }

    // ── executableURL ──────────────────────────────────────────

    #[test]
    fn test_executable_url_nil() {
        assert_eq!(shim_nstask_executable_url(0, 0), 0);
    }

    #[test]
    fn test_executable_url_no_path() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_executable_url(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_executable_url_returns_nsurl() {
        let obj = alloc_task();
        let path = make_string("/usr/bin/echo");
        shim_nstask_set_launch_path(obj, 0, path);
        let url = shim_nstask_executable_url(obj, 0);
        assert_ne!(url, 0);
        let url_path = crate::nsurl::shim_nsurl_path(url, 0);
        assert_string_eq(url_path, "/usr/bin/echo");
        free_obj(obj);
    }

    // ── description ────────────────────────────────────────────

    #[test]
    fn test_description_nil() {
        assert_eq!(shim_nstask_description(0, 0), 0);
    }

    #[test]
    fn test_description_no_path() {
        let obj = alloc_task();
        let desc = shim_nstask_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSTask: (null) pid=0>");
        free_obj(desc);
        free_obj(obj);
    }

    #[test]
    fn test_description_with_path() {
        let obj = alloc_task();
        let path = make_string("/usr/bin/echo");
        shim_nstask_set_launch_path(obj, 0, path);
        let desc = shim_nstask_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSTask: /usr/bin/echo pid=0>");
        free_obj(desc);
        free_obj(path);
        free_obj(obj);
    }

    // ── launch (nil / no-path early-return only — no real subprocess) ──

    #[test]
    fn test_launch_nil() {
        assert_eq!(shim_nstask_launch(0, 0), 0);
    }

    #[test]
    fn test_launch_no_path_returns_zero() {
        let obj = alloc_task();
        // launch_path is 0 → extract_cfstring_cstr returns None → early return 0
        assert_eq!(shim_nstask_launch(obj, 0), 0);
        // pid should still be 0 (no spawn happened)
        assert_eq!(shim_nstask_process_identifier(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_launch_inherits_host_environment_when_task_env_is_nil() {
        let obj = alloc_task();
        let path = make_string("/usr/bin/printenv");
        let key = format!("NSTASK_TEST_INHERIT_ENV_{}", std::process::id());
        let value = "burdettie";
        let key_obj = make_string(&key);
        let args = crate::nsarray::make_nsarray(&[key_obj]);
        let stdout_pipe = crate::nspipe::shim_nspipe_pipe(0, 0);

        std::env::set_var(&key, value);

        shim_nstask_set_launch_path(obj, 0, path);
        shim_nstask_set_arguments(obj, 0, args);
        shim_nstask_set_stdout(obj, 0, stdout_pipe);

        assert_eq!(shim_nstask_launch(obj, 0), 0);
        shim_nstask_wait_until_exit(obj, 0);

        let read_handle = crate::nspipe::shim_nspipe_file_handle_for_reading(stdout_pipe, 0);
        let data = crate::nsfilehandle::shim_nsfilehandle_read_data_to_end(read_handle, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0) as usize;
        let bytes = crate::nsdata::shim_nsdata_bytes(data, 0);
        let output = if len == 0 || bytes == 0 {
            String::new()
        } else {
            let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, len) };
            String::from_utf8_lossy(slice).trim().to_string()
        };

        assert_eq!(shim_nstask_termination_status(obj, 0), 0);
        assert_eq!(output, value);

        std::env::remove_var(&key);

        free_obj(data);
        free_obj(stdout_pipe);
        free_obj(args);
        free_obj(key_obj);
        free_obj(path);
        free_obj(obj);
    }

    // ── launchAndReturnError (nil / no-path only) ──────────────

    #[test]
    fn test_launch_and_return_error_nil() {
        assert_eq!(shim_nstask_launch_and_return_error(0, 0, 0), 0);
    }

    #[test]
    fn test_launch_and_return_error_no_path_writes_error() {
        let obj = alloc_task();
        let err_buf = alloc_u64_buf(1);
        let result = shim_nstask_launch_and_return_error(obj, 0, err_buf);
        assert_eq!(result, 0); // NO — launch failed
                               // error_out should have been written with a non-zero NSError
        let err = read_field(err_buf, 0);
        assert_ne!(err, 0, "expected NSError to be written");
        free_buf(err_buf);
        free_obj(obj);
    }

    #[test]
    fn test_launch_and_return_error_no_path_no_errout() {
        let obj = alloc_task();
        let result = shim_nstask_launch_and_return_error(obj, 0, 0);
        assert_eq!(result, 0);
        free_obj(obj);
    }

    // ── waitUntilExit (nil / no-pid early-return only) ─────────

    #[test]
    fn test_wait_until_exit_nil() {
        assert_eq!(shim_nstask_wait_until_exit(0, 0), 0);
    }

    #[test]
    fn test_wait_until_exit_no_pid() {
        let obj = alloc_task();
        // pid is 0 → returns early without calling waitpid
        assert_eq!(shim_nstask_wait_until_exit(obj, 0), 0);
        free_obj(obj);
    }

    // ── launchedTaskWithLaunchPath:arguments: (nil path) ───────

    #[test]
    fn test_launched_with_path_nil_path() {
        // path=0 → launch fails gracefully (extract_cfstring_cstr returns None)
        let obj = shim_nstask_launched_with_path(0, 0, 0, 0);
        assert_ne!(obj, 0); // task object is still allocated
        assert_eq!(shim_nstask_process_identifier(obj, 0), 0);
        free_obj(obj);
    }

    // ── roundtrip: set all fields then read them back ──────────

    #[test]
    fn test_roundtrip_all_fields() {
        let obj = alloc_task();
        let path = make_string("/bin/ls");
        let dir = make_string("/tmp");

        shim_nstask_set_launch_path(obj, 0, path);
        shim_nstask_set_arguments(obj, 0, 0xA);
        shim_nstask_set_environment(obj, 0, 0xB);
        shim_nstask_set_stdout(obj, 0, 0xC);
        shim_nstask_set_stdin(obj, 0, 0xD);
        shim_nstask_set_stderr(obj, 0, 0xE);
        shim_nstask_set_current_dir(obj, 0, dir);

        assert_eq!(shim_nstask_launch_path(obj, 0), path);
        assert_eq!(shim_nstask_arguments(obj, 0), 0xA);
        assert_eq!(shim_nstask_environment(obj, 0), 0xB);
        assert_eq!(shim_nstask_get_stdout(obj, 0), 0xC);
        assert_eq!(shim_nstask_get_stdin(obj, 0), 0xD);
        assert_eq!(shim_nstask_get_stderr(obj, 0), 0xE);
        assert_eq!(shim_nstask_current_dir(obj, 0), dir);

        free_obj(path);
        free_obj(dir);
        free_obj(obj);
    }

    // ── defaults after alloc (all fields zero except exit_status) ──

    #[test]
    fn test_defaults_after_alloc() {
        let obj = alloc_task();
        assert_eq!(shim_nstask_launch_path(obj, 0), 0);
        assert_eq!(shim_nstask_arguments(obj, 0), 0);
        assert_eq!(shim_nstask_environment(obj, 0), 0);
        assert_eq!(shim_nstask_get_stdout(obj, 0), 0);
        assert_eq!(shim_nstask_get_stdin(obj, 0), 0);
        assert_eq!(shim_nstask_get_stderr(obj, 0), 0);
        assert_eq!(shim_nstask_current_dir(obj, 0), 0);
        assert_eq!(shim_nstask_termination_status(obj, 0), -1);
        assert_eq!(shim_nstask_process_identifier(obj, 0), 0);
        free_obj(obj);
    }
}
