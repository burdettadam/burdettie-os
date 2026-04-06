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
    pub fn from_launch_data(dict: crate::launch_data::LaunchDataT) -> Option<Self> {
        use crate::constants::*;
        use crate::launch_data::*;

        let label = dict_get_string_value(dict, LAUNCH_JOBKEY_LABEL)?;
        let mut job = Job::new(&label);

        job.program = dict_get_string_value(dict, LAUNCH_JOBKEY_PROGRAM);
        job.disabled = dict_get_bool_value(dict, LAUNCH_JOBKEY_DISABLED).unwrap_or(false);
        job.run_at_load = dict_get_bool_value(dict, LAUNCH_JOBKEY_RUNATLOAD).unwrap_or(false);
        job.working_directory = dict_get_string_value(dict, LAUNCH_JOBKEY_WORKINGDIRECTORY);
        job.stdout_path = dict_get_string_value(dict, LAUNCH_JOBKEY_STANDARDOUTPATH);
        job.stderr_path = dict_get_string_value(dict, LAUNCH_JOBKEY_STANDARDERRORPATH);
        job.stdin_path = dict_get_string_value(dict, LAUNCH_JOBKEY_STANDARDINPATH);
        job.process_type = dict_get_string_value(dict, LAUNCH_JOBKEY_PROCESSTYPE);
        job.transactions_enabled =
            dict_get_bool_value(dict, LAUNCH_JOBKEY_ENABLETRANSACTIONS).unwrap_or(false);

        if let Some(interval) = dict_get_integer_value(dict, LAUNCH_JOBKEY_STARTINTERVAL) {
            job.start_interval = interval.max(0) as u32;
        }

        // Parse ProgramArguments array
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS).unwrap_or_default();
        let args = dict_lookup(dict, c_key.as_ptr());
        if !args.is_null() && get_type(args) == LaunchDataType::Array as u32 {
            let count = array_get_count(args);
            for i in 0..count {
                let elem = array_get_index(args, i);
                if !elem.is_null() {
                    let s = get_string(elem);
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
        let env_dict = dict_lookup(dict, c_key.as_ptr());
        if !env_dict.is_null() && get_type(env_dict) == LaunchDataType::Dictionary as u32 {
            // Use iterate to collect key-value pairs
            unsafe extern "C" fn collect_env(
                val: LaunchDataT,
                key: *const libc::c_char,
                ctx: *mut libc::c_void,
            ) {
                let map = unsafe { &mut *(ctx as *mut HashMap<String, String>) };
                if !key.is_null() && !val.is_null() {
                    let k = unsafe { std::ffi::CStr::from_ptr(key) };
                    let s = get_string(val);
                    if !s.is_null() {
                        if let (Ok(ks), Ok(vs)) = (
                            k.to_str(),
                            unsafe { std::ffi::CStr::from_ptr(s) }.to_str(),
                        ) {
                            map.insert(ks.to_string(), vs.to_string());
                        }
                    }
                }
            }
            let mut env_map = HashMap::new();
            dict_iterate(
                env_dict,
                collect_env,
                &mut env_map as *mut HashMap<String, String> as *mut libc::c_void,
            );
            job.environment = env_map;
        }

        // Parse MachServices dict — just record the service names
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_MACHSERVICES).unwrap_or_default();
        let mach_dict = dict_lookup(dict, c_key.as_ptr());
        if !mach_dict.is_null() && get_type(mach_dict) == LaunchDataType::Dictionary as u32 {
            unsafe extern "C" fn collect_mach(
                _val: LaunchDataT,
                key: *const libc::c_char,
                ctx: *mut libc::c_void,
            ) {
                let map = unsafe { &mut *(ctx as *mut HashMap<String, u32>) };
                if !key.is_null() {
                    if let Ok(ks) = unsafe { std::ffi::CStr::from_ptr(key) }.to_str() {
                        // Port 0 = not yet checked in
                        map.insert(ks.to_string(), 0);
                    }
                }
            }
            let mut mach_map = HashMap::new();
            dict_iterate(
                mach_dict,
                collect_mach,
                &mut mach_map as *mut HashMap<String, u32> as *mut libc::c_void,
            );
            job.mach_services = mach_map;
        }

        // Parse KeepAlive — can be a simple bool or a dict
        let c_key = std::ffi::CString::new(LAUNCH_JOBKEY_KEEPALIVE).unwrap_or_default();
        let ka = dict_lookup(dict, c_key.as_ptr());
        if !ka.is_null() {
            match get_type(ka) {
                t if t == LaunchDataType::Bool as u32 => {
                    job.keep_alive = get_bool(ka);
                }
                t if t == LaunchDataType::Dictionary as u32 => {
                    // Any non-empty KeepAlive dict means keep-alive is conceptually on
                    job.keep_alive = dict_get_count(ka) > 0;
                }
                _ => {}
            }
        }

        Some(job)
    }

    /// Convert this job back to a launch_data_t dictionary.
    pub fn to_launch_data(&self) -> crate::launch_data::LaunchDataT {
        use crate::constants::*;
        use crate::launch_data::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);

        dict_insert(dict, new_string(
            std::ffi::CString::new(self.label.as_str()).unwrap_or_default().as_ptr(),
        ), std::ffi::CString::new(LAUNCH_JOBKEY_LABEL).unwrap_or_default().as_ptr());

        if let Some(ref prog) = self.program {
            dict_insert(dict, new_string(
                std::ffi::CString::new(prog.as_str()).unwrap_or_default().as_ptr(),
            ), std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAM).unwrap_or_default().as_ptr());
        }

        if self.pid != 0 {
            dict_insert(dict, new_integer(i64::from(self.pid)),
                std::ffi::CString::new(LAUNCH_JOBKEY_PID).unwrap_or_default().as_ptr());
        }

        dict_insert(dict, new_integer(i64::from(self.last_exit_status)),
            std::ffi::CString::new(LAUNCH_JOBKEY_LASTEXITSTATUS).unwrap_or_default().as_ptr());

        if !self.program_arguments.is_empty() {
            let arr = alloc(LaunchDataType::Array as u32);
            for (i, arg) in self.program_arguments.iter().enumerate() {
                let s = new_string(
                    std::ffi::CString::new(arg.as_str()).unwrap_or_default().as_ptr(),
                );
                array_set_index(arr, s, i);
            }
            dict_insert(dict, arr,
                std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS).unwrap_or_default().as_ptr());
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
        let label = unique_label("dup");
        assert!(submit_job(Job::new(&label)).is_ok());
        assert!(submit_job(Job::new(&label)).is_err());
    }

    #[test]
    fn remove_job_works() {
        let label = unique_label("rm");
        assert!(submit_job(Job::new(&label)).is_ok());
        assert!(remove_job(&label).is_ok());
        assert!(get_job(&label).is_none());
        assert!(remove_job(&label).is_err());
    }

    #[test]
    fn checkin_flow() {
        let label = unique_label("daemon");
        submit_job(Job::new(&label)).unwrap();
        let job = checkin(&label);
        assert!(job.is_some());
        assert_eq!(get_checked_in_label().as_deref(), Some(label.as_str()));
    }

    #[test]
    fn mach_service_registration() {
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
        use crate::launch_data::*;
        use crate::constants::*;

        let dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(dict, new_string(
            std::ffi::CString::new("com.test.plist").unwrap().as_ptr(),
        ), std::ffi::CString::new(LAUNCH_JOBKEY_LABEL).unwrap().as_ptr());
        dict_insert(dict, new_string(
            std::ffi::CString::new("/bin/echo").unwrap().as_ptr(),
        ), std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAM).unwrap().as_ptr());
        dict_insert(dict, new_bool(true),
            std::ffi::CString::new(LAUNCH_JOBKEY_RUNATLOAD).unwrap().as_ptr());

        // ProgramArguments
        let args = alloc(LaunchDataType::Array as u32);
        array_set_index(args, new_string(
            std::ffi::CString::new("echo").unwrap().as_ptr(),
        ), 0);
        array_set_index(args, new_string(
            std::ffi::CString::new("hello").unwrap().as_ptr(),
        ), 1);
        dict_insert(dict, args,
            std::ffi::CString::new(LAUNCH_JOBKEY_PROGRAMARGUMENTS).unwrap().as_ptr());

        let job = Job::from_launch_data(dict).unwrap();
        assert_eq!(job.label, "com.test.plist");
        assert_eq!(job.program.as_deref(), Some("/bin/echo"));
        assert!(job.run_at_load);
        assert_eq!(job.program_arguments, vec!["echo", "hello"]);

        // Roundtrip back to launch_data
        let dict2 = job.to_launch_data();
        assert_eq!(dict_get_string_value(dict2, LAUNCH_JOBKEY_LABEL).as_deref(), Some("com.test.plist"));

        free(dict);
        free(dict2);
    }
}
