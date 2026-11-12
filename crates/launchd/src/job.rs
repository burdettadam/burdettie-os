//! Job and service registry — the core of launchd's process management.
//!
//! Jobs are registered either via `launch_msg(SubmitJob)` or directly
//! via the Rust API.  Each job has a label, program/arguments, environment,
//! and associated Mach services.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// A registered launchd job.
#[derive(Debug, Clone)]
pub struct Job {
    /// The job label (e.g. "com.apple.Finder").
    pub label: String,
    /// Program path (from LAUNCH_JOBKEY_PROGRAM).
    pub program: Option<String>,
    /// Program arguments (from LAUNCH_JOBKEY_PROGRAMARGUMENTS).
    pub program_arguments: Vec<String>,
    /// Environment variables.
    pub environment: HashMap<String, String>,
    /// Working directory.
    pub working_directory: Option<String>,
    /// Whether the job is disabled.
    pub disabled: bool,
    /// Whether to run at load.
    pub run_at_load: bool,
    /// KeepAlive flag.
    pub keep_alive: bool,
    /// Mach services advertised by this job: name → checked-in port.
    pub mach_services: HashMap<String, u32>,
    /// Socket descriptors for socket-activated services.
    pub sockets: HashMap<String, Vec<i32>>,
    /// PID if currently running, 0 if not.
    pub pid: i32,
    /// Last exit status.
    pub last_exit_status: i32,
    /// Start interval (seconds), 0 = not set.
    pub start_interval: u32,
    /// Idle timeout (seconds), 0 = not set.
    pub idle_timeout: u32,
    /// Transactions enabled.
    pub transactions_enabled: bool,
    /// Process type.
    pub process_type: Option<String>,
    /// StandardOutPath.
    pub stdout_path: Option<String>,
    /// StandardErrorPath.
    pub stderr_path: Option<String>,
    /// StandardInPath.
    pub stdin_path: Option<String>,
}

impl Job {
    /// Create a new job with just a label.
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            program: None,
            program_arguments: Vec::new(),
            environment: HashMap::new(),
            working_directory: None,
            disabled: false,
            run_at_load: false,
            keep_alive: false,
            mach_services: HashMap::new(),
            sockets: HashMap::new(),
            pid: 0,
            last_exit_status: 0,
            start_interval: 0,
            idle_timeout: 0,
            transactions_enabled: false,
            process_type: None,
            stdout_path: None,
            stderr_path: None,
            stdin_path: None,
        }
    }

    /// Parse a job from a launch_data_t dictionary.
    ///
    /// # Safety
    /// `dict` must be null or a live, aligned `LaunchData` handle whose entire
    /// value tree is exclusively accessible for this call. Lookup helpers borrow
    /// the containers mutably. The caller retains ownership of the handle.
    pub unsafe fn from_launch_data(dict: crate::launch_data::LaunchDataT) -> Option<Self> {
        use crate::constants::*;
        use crate::launch_data::*;

        let label = unsafe { dict_get_string_value(dict, LAUNCH_JOBKEY_LABEL) }?;
        let mut job = Job::new(&label);

        job.program = unsafe { dict_get_string_value(dict, LAUNCH_JOBKEY_PROGRAM) };
        job.disabled =
            unsafe { dict_get_bool_value(dict, LAUNCH_JOBKEY_DISABLED) }.unwrap_or(false);
        job.run_at_load =
            unsafe { dict_get_bool_value(dict, LAUNCH_JOBKEY_RUNATLOAD) }.unwrap_or(false);
        job.working_directory =
            unsafe { dict_get_string_value(dict, LAUNCH_JOBKEY_WORKINGDIRECTORY) };
        job.stdout_path = unsafe { dict_get_string_value(dict, LAUNCH_JOBKEY_STANDARDOUTPATH) };
        job.stderr_path = unsafe { dict_get_string_value(dict, LAUNCH_JOBKEY_STANDARDERRORPATH) };
        job.stdin_path = unsafe { dict_get_string_value(dict, LAUNCH_JOBKEY_STANDARDINPATH) };
        job.process_type = unsafe { dict_get_string_value(dict, LAUNCH_JOBKEY_PROCESSTYPE) };
        job.transactions_enabled =
            unsafe { dict_get_bool_value(dict, LAUNCH_JOBKEY_ENABLETRANSACTIONS) }.unwrap_or(false);

        if let Some(interval) = unsafe { dict_get_integer_value(dict, LAUNCH_JOBKEY_STARTINTERVAL) }
        {
            job.start_interval = interval.max(0) as u32;
        }

        // Parse ProgramArguments array
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS).unwrap_or_default();
        let args = unsafe { dict_lookup(dict, c_key.as_ptr()) };
        if !args.is_null() && unsafe { get_type(args) } == LaunchDataType::Array as u32 {
            let count = unsafe { array_get_count(args) };
            for i in 0..count {
                let elem = unsafe { array_get_index(args, i) };
                if !elem.is_null() {
                    let s = unsafe { get_string(elem) };
                    if !s.is_null() {
                        if let Ok(cs) = unsafe { std::ffi::CStr::from_ptr(s) }.to_str() {
                            job.program_arguments.push(cs.to_string());
                        }
                    }
                }
            }
        }

        // Parse EnvironmentVariables dict
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_ENVIRONMENTVARIABLES).unwrap_or_default();
        let env_dict = unsafe { dict_lookup(dict, c_key.as_ptr()) };
        if !env_dict.is_null() && unsafe { get_type(env_dict) } == LaunchDataType::Dictionary as u32
        {
            // Use iterate to collect key-value pairs
            unsafe extern "C" fn collect_env(
                val: LaunchDataT,
                key: *const libc::c_char,
                ctx: *mut libc::c_void,
            ) {
                // SAFETY: The synchronous dict_iterate call below supplies its
                // live local map and borrowed key/value for this callback only.
                let map = unsafe { &mut *(ctx as *mut HashMap<String, String>) };
                if !key.is_null() && !val.is_null() {
                    let k = unsafe { std::ffi::CStr::from_ptr(key) };
                    let s = unsafe { get_string(val) };
                    if !s.is_null() {
                        if let (Ok(ks), Ok(vs)) =
                            (k.to_str(), unsafe { std::ffi::CStr::from_ptr(s) }.to_str())
                        {
                            map.insert(ks.to_string(), vs.to_string());
                        }
                    }
                }
            }
            let mut env_map = HashMap::new();
            // SAFETY: dict remains exclusively borrowed by this parser, and
            // collect_env only copies data into the live local env_map.
            unsafe {
                dict_iterate(
                    env_dict,
                    collect_env,
                    &mut env_map as *mut HashMap<String, String> as *mut libc::c_void,
                )
            };
            job.environment = env_map;
        }

        // Parse MachServices dict — just record the service names
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_MACHSERVICES).unwrap_or_default();
        let mach_dict = unsafe { dict_lookup(dict, c_key.as_ptr()) };
        if !mach_dict.is_null()
            && unsafe { get_type(mach_dict) } == LaunchDataType::Dictionary as u32
        {
            unsafe extern "C" fn collect_mach(
                _val: LaunchDataT,
                key: *const libc::c_char,
                ctx: *mut libc::c_void,
            ) {
                // SAFETY: The synchronous iteration supplies the live local
                // mach_map as context and a key valid for this callback only.
                let map = unsafe { &mut *(ctx as *mut HashMap<String, u32>) };
                if !key.is_null() {
                    if let Ok(ks) = unsafe { std::ffi::CStr::from_ptr(key) }.to_str() {
                        // Port 0 = not yet checked in
                        map.insert(ks.to_string(), 0);
                    }
                }
            }
            let mut mach_map = HashMap::new();
            // SAFETY: The callback never touches its parent dictionary and
            // stores owned key strings in the live local mach_map.
            unsafe {
                dict_iterate(
                    mach_dict,
                    collect_mach,
                    &mut mach_map as *mut HashMap<String, u32> as *mut libc::c_void,
                )
            };
            job.mach_services = mach_map;
        }

        // Parse KeepAlive — can be a simple bool or a dict
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_KEEPALIVE).unwrap_or_default();
        let ka = unsafe { dict_lookup(dict, c_key.as_ptr()) };
        if !ka.is_null() {
            match unsafe { get_type(ka) } {
                t if t == LaunchDataType::Bool as u32 => {
                    job.keep_alive = unsafe { get_bool(ka) };
                }
                t if t == LaunchDataType::Dictionary as u32 => {
                    // Any non-empty KeepAlive dict means keep-alive is conceptually on
                    job.keep_alive = unsafe { dict_get_count(ka) } > 0;
                }
                _ => {}
            }
        }

        Some(job)
    }

    /// Convert this job back to a launch_data_t dictionary.
    pub fn to_launch_data(&self) -> crate::launch_data::LaunchDataT {
        // SAFETY: All handles below are newly allocated and transferred once into the
        // returned dictionary. String temporaries live through calls that copy them.
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);

        unsafe {
            dict_insert(
                dict,
                new_string(
                    std::ffi::CString::new(self.label.as_str())
                        .unwrap_or_default()
                        .as_ptr(),
                ),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap_or_default()
                    .as_ptr(),
            )
        };

        if let Some(ref prog) = self.program {
            unsafe {
                dict_insert(
                    dict,
                    new_string(
                        std::ffi::CString::new(prog.as_str())
                            .unwrap_or_default()
                            .as_ptr(),
                    ),
                    std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAM)
                        .unwrap_or_default()
                        .as_ptr(),
                )
            };
        }

        if self.pid != 0 {
            unsafe {
                dict_insert(
                    dict,
                    new_integer(i64::from(self.pid)),
                    std::ffi::CString::new(LAUNCH_JOBKEY_PID)
                        .unwrap_or_default()
                        .as_ptr(),
                )
            };
        }

        unsafe {
            dict_insert(
                dict,
                new_integer(i64::from(self.last_exit_status)),
                std::ffi::CString::new(LAUNCH_JOBKEY_LASTEXITSTATUS)
                    .unwrap_or_default()
                    .as_ptr(),
            )
        };

        if !self.program_arguments.is_empty() {
            let arr = alloc(LaunchDataType::Array as u32);
            for (i, arg) in self.program_arguments.iter().enumerate() {
                let s = unsafe {
                    new_string(
                        std::ffi::CString::new(arg.as_str())
                            .unwrap_or_default()
                            .as_ptr(),
                    )
                };
                unsafe { array_set_index(arr, s, i) };
            }
            unsafe {
                dict_insert(
                    dict,
                    arr,
                    std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS)
                        .unwrap_or_default()
                        .as_ptr(),
                )
            };
        }

        dict
    }
}

// ── Global job registry ─────────────────────────────────────────────

struct JobRegistryInner {
    /// Jobs by label.
    jobs: HashMap<String, Job>,
    /// The label of the "current" job (the one that checked in).
    checked_in_label: Option<String>,
}

static JOB_REGISTRY: OnceLock<Mutex<JobRegistryInner>> = OnceLock::new();

fn with_registry<R>(f: impl FnOnce(&mut JobRegistryInner) -> R) -> R {
    let mutex = JOB_REGISTRY.get_or_init(|| {
        Mutex::new(JobRegistryInner {
            jobs: HashMap::new(),
            checked_in_label: None,
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

/// Submit a new job to the registry.
pub fn submit_job(job: Job) -> Result<(), &'static str> {
    with_registry(|reg| {
        if reg.jobs.contains_key(&job.label) {
            return Err("job already exists");
        }
        log::debug!("launchd: submitted job '{}'", job.label);
        reg.jobs.insert(job.label.clone(), job);
        Ok(())
    })
}

/// Remove a job by label.
pub fn remove_job(label: &str) -> Result<(), &'static str> {
    with_registry(|reg| {
        if reg.jobs.remove(label).is_some() {
            log::debug!("launchd: removed job '{}'", label);
            Ok(())
        } else {
            Err("job not found")
        }
    })
}

/// Get job info by label.
pub fn get_job(label: &str) -> Option<Job> {
    with_registry(|reg| reg.jobs.get(label).cloned())
}

/// Get all job labels.
pub fn get_all_job_labels() -> Vec<String> {
    with_registry(|reg| reg.jobs.keys().cloned().collect())
}

/// Perform a checkin for the current process job.
pub fn checkin(label: &str) -> Option<Job> {
    with_registry(|reg| {
        if let Some(job) = reg.jobs.get(label) {
            reg.checked_in_label = Some(label.to_string());
            log::debug!("launchd: job '{}' checked in", label);
            Some(job.clone())
        } else {
            None
        }
    })
}

/// Get the currently checked-in job label.
pub fn get_checked_in_label() -> Option<String> {
    with_registry(|reg| reg.checked_in_label.clone())
}

/// Register a Mach service port for a job.
pub fn register_mach_service(label: &str, service_name: &str, port: u32) {
    with_registry(|reg| {
        if let Some(job) = reg.jobs.get_mut(label) {
            job.mach_services.insert(service_name.to_string(), port);
            log::debug!(
                "launchd: registered Mach service '{}' → port {:#x} for job '{}'",
                service_name,
                port,
                label
            );
        }
    });
}

/// Look up a Mach service port across all jobs.
pub fn look_up_mach_service(service_name: &str) -> Option<u32> {
    with_registry(|reg| {
        for job in reg.jobs.values() {
            if let Some(&port) = job.mach_services.get(service_name) {
                if port != 0 {
                    return Some(port);
                }
            }
        }
        None
    })
}

/// Set the PID for a running job.
pub fn set_job_pid(label: &str, pid: i32) {
    with_registry(|reg| {
        if let Some(job) = reg.jobs.get_mut(label) {
            job.pid = pid;
        }
    });
}

/// Set the last exit status for a job.
pub fn set_job_exit_status(label: &str, status: i32) {
    with_registry(|reg| {
        if let Some(job) = reg.jobs.get_mut(label) {
            job.last_exit_status = status;
            job.pid = 0;
        }
    });
}

/// Reset the job registry (for testing).
pub fn reset_registry() {
    with_registry(|reg| {
        reg.jobs.clear();
        reg.checked_in_label = None;
    });
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    // SAFETY: The tests own their launch-data trees, transfer each child once,
    // and use borrowed children only while their owning container is alive.

    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Generate a unique job label to avoid inter-test interference.
    fn unique_label(prefix: &str) -> String {
        static CTR: AtomicU32 = AtomicU32::new(0);
        let n = CTR.fetch_add(1, Ordering::Relaxed);
        format!("com.job.{}.{}", prefix, n)
    }

    #[test]
    fn submit_and_get_job() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("myapp");
        let mut job = Job::new(&label);
        job.program = Some("/usr/bin/myapp".to_string());
        job.run_at_load = true;
        job.program_arguments = vec!["myapp".to_string(), "--flag".to_string()];

        assert!(submit_job(job).is_ok());
        let retrieved = get_job(&label);
        assert!(retrieved.is_some());
        let j = retrieved.unwrap();
        assert_eq!(j.label, label);
        assert_eq!(j.program.as_deref(), Some("/usr/bin/myapp"));
        assert!(j.run_at_load);
        assert_eq!(j.program_arguments.len(), 2);
    }

    #[test]
    fn duplicate_submit_fails() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("dup");
        assert!(submit_job(Job::new(&label)).is_ok());
        assert!(submit_job(Job::new(&label)).is_err());
    }

    #[test]
    fn remove_job_works() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("rm");
        assert!(submit_job(Job::new(&label)).is_ok());
        assert!(remove_job(&label).is_ok());
        assert!(get_job(&label).is_none());
        assert!(remove_job(&label).is_err());
    }

    #[test]
    fn checkin_flow() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("daemon");
        submit_job(Job::new(&label)).unwrap();
        let job = checkin(&label);
        assert!(job.is_some());
        assert_eq!(get_checked_in_label().as_deref(), Some(label.as_str()));
    }

    #[test]
    fn mach_service_registration() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("svc");
        let svc_name = format!("{}.port", label);
        let mut job = Job::new(&label);
        job.mach_services.insert(svc_name.clone(), 0);
        submit_job(job).unwrap();

        // Before registration, port is 0 (not yet checked in)
        assert!(look_up_mach_service(&svc_name).is_none());

        // Register port
        register_mach_service(&label, &svc_name, 0x2000);
        assert_eq!(look_up_mach_service(&svc_name), Some(0x2000));
    }

    #[test]
    fn get_all_labels() {
        let _state_guard = crate::lock_test_state();
        let l1 = unique_label("ga");
        let l2 = unique_label("gb");
        let l3 = unique_label("gc");
        submit_job(Job::new(&l1)).unwrap();
        submit_job(Job::new(&l2)).unwrap();
        submit_job(Job::new(&l3)).unwrap();
        let labels = get_all_job_labels();
        // All three must be present (other tests may add more)
        assert!(labels.contains(&l1));
        assert!(labels.contains(&l2));
        assert!(labels.contains(&l3));
    }

    #[test]
    fn job_pid_and_exit_status() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("proc");
        submit_job(Job::new(&label)).unwrap();
        set_job_pid(&label, 1234);
        assert_eq!(get_job(&label).unwrap().pid, 1234);
        set_job_exit_status(&label, 42);
        let j = get_job(&label).unwrap();
        assert_eq!(j.pid, 0);
        assert_eq!(j.last_exit_status, 42);
    }

    #[test]
    fn from_launch_data_roundtrip() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(std::ffi::CString::new("com.test.plist").unwrap().as_ptr()),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_string(std::ffi::CString::new("/bin/echo").unwrap().as_ptr()),
                std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAM)
                    .unwrap()
                    .as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_bool(true),
                std::ffi::CString::new(LAUNCH_JOBKEY_RUNATLOAD)
                    .unwrap()
                    .as_ptr(),
            )
        };

        // ProgramArguments
        let args = alloc(LaunchDataType::Array as u32);
        unsafe {
            array_set_index(
                args,
                new_string(std::ffi::CString::new("echo").unwrap().as_ptr()),
                0,
            )
        };
        unsafe {
            array_set_index(
                args,
                new_string(std::ffi::CString::new("hello").unwrap().as_ptr()),
                1,
            )
        };
        unsafe {
            dict_insert(
                dict,
                args,
                std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert_eq!(job.label, "com.test.plist");
        assert_eq!(job.program.as_deref(), Some("/bin/echo"));
        assert!(job.run_at_load);
        assert_eq!(job.program_arguments, vec!["echo", "hello"]);

        // Roundtrip back to launch_data
        let dict2 = job.to_launch_data();
        assert_eq!(
            unsafe { dict_get_string_value(dict2, LAUNCH_JOBKEY_LABEL) }.as_deref(),
            Some("com.test.plist")
        );

        unsafe { free(dict) };
        unsafe { free(dict2) };
    }

    // ── from_launch_data edge cases ──────────────────────────────

    #[test]
    fn from_launch_data_missing_label_returns_none() {
        use crate::launch_data::*;
        let dict = alloc(LaunchDataType::Dictionary as u32);
        // No Label key — should return None
        assert!(unsafe { Job::from_launch_data(dict) }.is_none());
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_null_returns_none() {
        assert!(unsafe { Job::from_launch_data(std::ptr::null_mut()) }.is_none());
    }

    #[test]
    fn from_launch_data_non_dict_returns_none() {
        use crate::launch_data::*;
        // An integer handle is not a dict
        let int_handle = new_integer(42);
        assert!(unsafe { Job::from_launch_data(int_handle) }.is_none());
        unsafe { free(int_handle) };
    }

    #[test]
    fn from_launch_data_with_environment_variables() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(std::ffi::CString::new("com.test.env").unwrap().as_ptr()),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let env = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                env,
                new_string(std::ffi::CString::new("/usr/local/bin").unwrap().as_ptr()),
                std::ffi::CString::new("PATH").unwrap().as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                env,
                new_string(std::ffi::CString::new("/tmp").unwrap().as_ptr()),
                std::ffi::CString::new("TMPDIR").unwrap().as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                env,
                std::ffi::CString::new(LAUNCH_JOBKEY_ENVIRONMENTVARIABLES)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert_eq!(job.environment.len(), 2);
        assert_eq!(job.environment.get("PATH").unwrap(), "/usr/local/bin");
        assert_eq!(job.environment.get("TMPDIR").unwrap(), "/tmp");
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_with_mach_services() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(std::ffi::CString::new("com.test.mach").unwrap().as_ptr()),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let mach = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                mach,
                new_bool(true),
                std::ffi::CString::new("com.test.mach.port")
                    .unwrap()
                    .as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                mach,
                new_bool(true),
                std::ffi::CString::new("com.test.mach.other")
                    .unwrap()
                    .as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                mach,
                std::ffi::CString::new(LAUNCH_JOBKEY_MACHSERVICES)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert_eq!(job.mach_services.len(), 2);
        // All ports start as 0 (not yet checked in)
        assert_eq!(job.mach_services.get("com.test.mach.port"), Some(&0));
        assert_eq!(job.mach_services.get("com.test.mach.other"), Some(&0));
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_keepalive_bool() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(std::ffi::CString::new("com.test.ka.bool").unwrap().as_ptr()),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_bool(true),
                std::ffi::CString::new(LAUNCH_JOBKEY_KEEPALIVE)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert!(job.keep_alive);
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_keepalive_dict() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(std::ffi::CString::new("com.test.ka.dict").unwrap().as_ptr()),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };
        let ka = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                ka,
                new_bool(true),
                std::ffi::CString::new("SuccessfulExit").unwrap().as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                ka,
                std::ffi::CString::new(LAUNCH_JOBKEY_KEEPALIVE)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert!(job.keep_alive); // non-empty dict → true
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_keepalive_empty_dict() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(
                    std::ffi::CString::new("com.test.ka.empty")
                        .unwrap()
                        .as_ptr(),
                ),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };
        let ka = alloc(LaunchDataType::Dictionary as u32);
        // Empty dict → keep_alive stays false
        unsafe {
            dict_insert(
                dict,
                ka,
                std::ffi::CString::new(LAUNCH_JOBKEY_KEEPALIVE)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert!(!job.keep_alive);
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_start_interval() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(
                    std::ffi::CString::new("com.test.interval")
                        .unwrap()
                        .as_ptr(),
                ),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_integer(300),
                std::ffi::CString::new(LAUNCH_JOBKEY_STARTINTERVAL)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert_eq!(job.start_interval, 300);
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_negative_start_interval_clamps_to_zero() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        unsafe {
            dict_insert(
                dict,
                new_string(std::ffi::CString::new("com.test.neg").unwrap().as_ptr()),
                std::ffi::CString::new(LAUNCH_JOBKEY_LABEL)
                    .unwrap()
                    .as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_integer(-5),
                std::ffi::CString::new(LAUNCH_JOBKEY_STARTINTERVAL)
                    .unwrap()
                    .as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert_eq!(job.start_interval, 0); // clamped via .max(0)
        unsafe { free(dict) };
    }

    #[test]
    fn from_launch_data_all_string_fields() {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);

        let mk = |s: &str| std::ffi::CString::new(s).unwrap();

        unsafe {
            dict_insert(
                dict,
                new_string(mk("com.test.paths").as_ptr()),
                mk(LAUNCH_JOBKEY_LABEL).as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_string(mk("/bin/ls").as_ptr()),
                mk(LAUNCH_JOBKEY_PROGRAM).as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_string(mk("/tmp").as_ptr()),
                mk(LAUNCH_JOBKEY_WORKINGDIRECTORY).as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_string(mk("/var/log/out.log").as_ptr()),
                mk(LAUNCH_JOBKEY_STANDARDOUTPATH).as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_string(mk("/var/log/err.log").as_ptr()),
                mk(LAUNCH_JOBKEY_STANDARDERRORPATH).as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_string(mk("/dev/null").as_ptr()),
                mk(LAUNCH_JOBKEY_STANDARDINPATH).as_ptr(),
            )
        };
        unsafe {
            dict_insert(
                dict,
                new_string(mk("Interactive").as_ptr()),
                mk(LAUNCH_JOBKEY_PROCESSTYPE).as_ptr(),
            )
        };
        unsafe { dict_insert(dict, new_bool(true), mk(LAUNCH_JOBKEY_DISABLED).as_ptr()) };
        unsafe {
            dict_insert(
                dict,
                new_bool(true),
                mk(LAUNCH_JOBKEY_ENABLETRANSACTIONS).as_ptr(),
            )
        };

        let job = unsafe { Job::from_launch_data(dict) }.unwrap();
        assert_eq!(job.program.as_deref(), Some("/bin/ls"));
        assert_eq!(job.working_directory.as_deref(), Some("/tmp"));
        assert_eq!(job.stdout_path.as_deref(), Some("/var/log/out.log"));
        assert_eq!(job.stderr_path.as_deref(), Some("/var/log/err.log"));
        assert_eq!(job.stdin_path.as_deref(), Some("/dev/null"));
        assert_eq!(job.process_type.as_deref(), Some("Interactive"));
        assert!(job.disabled);
        assert!(job.transactions_enabled);
        unsafe { free(dict) };
    }

    // ── to_launch_data ───────────────────────────────────────────

    #[test]
    fn to_launch_data_includes_pid_when_nonzero() {
        use crate::constants::*;
        use crate::launch_data::*;

        let mut job = Job::new("com.test.to_ld.pid");
        job.pid = 9876;
        let dict = job.to_launch_data();
        let pid_val = unsafe { dict_get_integer_value(dict, LAUNCH_JOBKEY_PID) };
        assert_eq!(pid_val, Some(9876));
        unsafe { free(dict) };
    }

    #[test]
    fn to_launch_data_omits_pid_when_zero() {
        use crate::constants::*;
        use crate::launch_data::*;

        let job = Job::new("com.test.to_ld.nopid");
        let dict = job.to_launch_data();
        // PID key should not be present
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_PID).unwrap();
        let val = unsafe { dict_lookup(dict, c_key.as_ptr()) };
        assert!(val.is_null());
        unsafe { free(dict) };
    }

    #[test]
    fn to_launch_data_includes_program_arguments() {
        use crate::constants::*;
        use crate::launch_data::*;

        let mut job = Job::new("com.test.to_ld.args");
        job.program_arguments = vec!["ls".into(), "-la".into(), "/tmp".into()];
        let dict = job.to_launch_data();

        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS).unwrap();
        let arr = unsafe { dict_lookup(dict, c_key.as_ptr()) };
        assert!(!arr.is_null());
        assert_eq!(unsafe { get_type(arr) }, LaunchDataType::Array as u32);
        assert_eq!(unsafe { array_get_count(arr) }, 3);

        let get_str = |idx: usize| -> String {
            let elem = unsafe { array_get_index(arr, idx) };
            let s = unsafe { get_string(elem) };
            unsafe { std::ffi::CStr::from_ptr(s) }
                .to_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(get_str(0), "ls");
        assert_eq!(get_str(1), "-la");
        assert_eq!(get_str(2), "/tmp");

        unsafe { free(dict) };
    }

    #[test]
    fn to_launch_data_omits_empty_program_arguments() {
        use crate::constants::*;
        use crate::launch_data::*;

        let job = Job::new("com.test.to_ld.noargs");
        let dict = job.to_launch_data();
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS).unwrap();
        let val = unsafe { dict_lookup(dict, c_key.as_ptr()) };
        assert!(val.is_null());
        unsafe { free(dict) };
    }

    #[test]
    fn to_launch_data_last_exit_status() {
        use crate::constants::*;
        use crate::launch_data::*;

        let mut job = Job::new("com.test.to_ld.exit");
        job.last_exit_status = -1;
        let dict = job.to_launch_data();
        let val = unsafe { dict_get_integer_value(dict, LAUNCH_JOBKEY_LASTEXITSTATUS) };
        assert_eq!(val, Some(-1));
        unsafe { free(dict) };
    }

    // ── Registry edge cases ──────────────────────────────────────

    #[test]
    fn checkin_nonexistent_returns_none() {
        let _state_guard = crate::lock_test_state();
        assert!(checkin("com.nonexistent.label.xyz").is_none());
    }

    #[test]
    fn set_pid_on_nonexistent_label_is_noop() {
        let _state_guard = crate::lock_test_state();
        set_job_pid("com.no.such.label.pid", 1234);
        // No panic, no error — silent no-op
    }

    #[test]
    fn set_exit_status_on_nonexistent_label_is_noop() {
        let _state_guard = crate::lock_test_state();
        set_job_exit_status("com.no.such.label.exit", 42);
    }

    #[test]
    fn register_mach_service_on_nonexistent_label_is_noop() {
        let _state_guard = crate::lock_test_state();
        register_mach_service("com.no.such.label.mach", "com.test.svc", 0x1234);
    }

    #[test]
    fn look_up_mach_service_not_found() {
        let _state_guard = crate::lock_test_state();
        assert!(look_up_mach_service("com.definitely.not.registered.ever").is_none());
    }

    #[test]
    fn mach_service_port_zero_not_returned() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("zero");
        let svc = format!("{}.svc", label);
        let mut job = Job::new(&label);
        job.mach_services.insert(svc.clone(), 0);
        submit_job(job).unwrap();

        // Port 0 means not yet checked in — look_up should return None
        assert!(look_up_mach_service(&svc).is_none());
    }

    #[test]
    fn multiple_jobs_mach_service_lookup() {
        let _state_guard = crate::lock_test_state();
        let l1 = unique_label("multi1");
        let l2 = unique_label("multi2");
        let svc1 = format!("{}.port", l1);
        let svc2 = format!("{}.port", l2);

        let mut j1 = Job::new(&l1);
        j1.mach_services.insert(svc1.clone(), 0);
        submit_job(j1).unwrap();
        register_mach_service(&l1, &svc1, 0x100);

        let mut j2 = Job::new(&l2);
        j2.mach_services.insert(svc2.clone(), 0);
        submit_job(j2).unwrap();
        register_mach_service(&l2, &svc2, 0x200);

        assert_eq!(look_up_mach_service(&svc1), Some(0x100));
        assert_eq!(look_up_mach_service(&svc2), Some(0x200));
    }

    #[test]
    fn exit_status_clears_pid() {
        let _state_guard = crate::lock_test_state();
        let label = unique_label("exitpid");
        submit_job(Job::new(&label)).unwrap();
        set_job_pid(&label, 5555);
        assert_eq!(get_job(&label).unwrap().pid, 5555);

        set_job_exit_status(&label, 1);
        let j = get_job(&label).unwrap();
        assert_eq!(j.pid, 0); // pid cleared
        assert_eq!(j.last_exit_status, 1);
    }

    #[test]
    fn job_new_defaults() {
        let j = Job::new("com.test.defaults");
        assert_eq!(j.label, "com.test.defaults");
        assert!(j.program.is_none());
        assert!(j.program_arguments.is_empty());
        assert!(j.environment.is_empty());
        assert!(j.working_directory.is_none());
        assert!(!j.disabled);
        assert!(!j.run_at_load);
        assert!(!j.keep_alive);
        assert!(j.mach_services.is_empty());
        assert!(j.sockets.is_empty());
        assert_eq!(j.pid, 0);
        assert_eq!(j.last_exit_status, 0);
        assert_eq!(j.start_interval, 0);
        assert_eq!(j.idle_timeout, 0);
        assert!(!j.transactions_enabled);
        assert!(j.process_type.is_none());
        assert!(j.stdout_path.is_none());
        assert!(j.stderr_path.is_none());
        assert!(j.stdin_path.is_none());
    }

    #[test]
    fn concurrent_submit_and_lookup() {
        let _state_guard = crate::lock_test_state();
        use std::sync::Arc;
        use std::thread;

        let labels: Vec<String> = (0..20).map(|_| unique_label("conc")).collect();
        let labels = Arc::new(labels);

        let handles: Vec<_> = (0..4)
            .map(|t| {
                let labels = labels.clone();
                thread::spawn(move || {
                    for i in 0..5 {
                        let idx = t * 5 + i;
                        let _ = submit_job(Job::new(&labels[idx]));
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        for label in labels.iter() {
            assert!(get_job(label).is_some(), "missing job: {}", label);
        }
    }
}
