//! Rust reimplementation of Apple's open-source **launchd** libraries.
//!
//! This crate provides the exact C API surface of:
//! - `<launch.h>` / `<launch_priv.h>` — `launch_data_t`, `launch_msg()`, `launch_get_fd()`
//! - `<servers/bootstrap.h>` / `<bootstrap_priv.h>` — `bootstrap_look_up()`, `bootstrap_check_in()`, etc.
//! - `<vproc.h>` / `<vproc_priv.h>` — `vproc_transaction_begin()`, `vproc_swap_integer()`, etc.
//!
//! Instead of IPC over Mach ports / Unix sockets to a real launchd daemon,
//! everything runs in-process with a thread-safe job registry and
//! bootstrap service namespace.  This is suitable for running macOS
//! binaries that expect launchd to be present (e.g. `launchctl`,
//! `xpcproxy`, daemons that check in at launch).
//!
//! # Modules
//!
//! - [`constants`] — All `LAUNCH_KEY_*`, `LAUNCH_JOBKEY_*`, `BOOTSTRAP_*`, `VPROC_GSK_*` constants
//! - [`launch_data`] — `launch_data_t` typed container (dictionary, array, fd, integer, real, bool, string, opaque, errno, machport)
//! - [`launch_msg`] — `launch_msg()` request routing and `launch_get_fd()`
//! - [`bootstrap`] — Bootstrap server port management
//! - [`vproc`] — Process transaction and lifecycle management
//! - [`job`] — Job registry (submit, remove, checkin, mach services)

pub mod bootstrap;
pub mod constants;
pub mod job;
pub mod launch_data;
pub mod launch_msg;
pub mod vproc;

/// Reset all launchd state for testing / runtime reuse.
pub fn reset_all_state() {
    job::reset_registry();
    bootstrap::reset_registry();
    vproc::reset_state();
}

/// Install well-known default services.
///
/// Call this during initialization to register standard launchd services
/// that macOS tools expect to exist.
pub fn install_defaults() {
    // Register common XPC/launchd service names in bootstrap
    bootstrap::register_service("com.apple.xpc.launchd", 0x2000);
    bootstrap::register_service("com.apple.xpc.launchd.domain-port", 0x2001);
    bootstrap::register_service("com.apple.distributed_notifications@Gv3", 0x2002);
    bootstrap::register_service("com.apple.system.notification_center", 0x2003);
    bootstrap::register_service("com.apple.CoreServices.coreservicesd", 0x2004);
    bootstrap::register_service("com.apple.SecurityServer", 0x2005);
    bootstrap::register_service("com.apple.windowserver.active", 0x2006);
    bootstrap::register_service("com.apple.pasteboard.1", 0x2007);
    log::debug!("launchd: installed default bootstrap services");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle() {
        // Install defaults (idempotent — already-registered services are skipped)
        install_defaults();

        // Bootstrap look up a default service
        let mut port: u32 = 0;
        let r = bootstrap::bootstrap_look_up(
            bootstrap::get_bootstrap_port(),
            c"com.apple.xpc.launchd".as_ptr(),
            &mut port,
        );
        assert_eq!(r, constants::BOOTSTRAP_SUCCESS);
        assert_ne!(port, 0);

        // Submit a job with a unique label
        use launch_data::*;
        let job_dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(job_dict, new_string(c"com.test.lifecycle.integ".as_ptr()), c"Label".as_ptr());
        dict_insert(job_dict, new_string(c"/bin/sleep".as_ptr()), c"Program".as_ptr());
        dict_insert(job_dict, new_bool(true), c"RunAtLoad".as_ptr());

        // Add MachServices
        let mach_dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(mach_dict, new_bool(true), c"com.test.lifecycle.integ.port".as_ptr());
        dict_insert(job_dict, mach_dict, c"MachServices".as_ptr());

        let req = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(req, job_dict, c"SubmitJob".as_ptr());
        let reply = launch_msg::launch_msg(req);
        // May fail with EEXIST from a previous run — both are fine
        let errno = get_errno(reply);
        assert!(errno == 0 || errno == libc::EEXIST);
        free(reply);
        free(req);

        // Check in the mach service via bootstrap (always succeeds even if auto-created)
        let mut svc_port: u32 = 0;
        let r = bootstrap::bootstrap_check_in(0, c"com.test.lifecycle.integ.port".as_ptr(), &mut svc_port);
        // Might already be checked in from another run
        assert!(r == constants::BOOTSTRAP_SUCCESS || r == constants::BOOTSTRAP_SERVICE_ACTIVE);

        // Vproc transactions
        let h = vproc::vproc_transaction_begin(std::ptr::null_mut());
        assert!(vproc::_vproc_transaction_count() >= 1);
        vproc::vproc_transaction_end(std::ptr::null_mut(), h);
    }

    /// This test verifies `reset_all_state()` clears all registries.
    /// It calls `reset_all_state()` which affects global state and may
    /// interfere with parallel tests. Run with `--test-threads=1` for
    /// strict isolation.
    #[test]
    fn reset_clears_specific_items() {
        let label = "com.test.reset_unique_82";
        let svc = "com.test.reset_svc_unique_82";

        let _ = job::remove_job(label);
        job::submit_job(job::Job::new(label)).unwrap();
        bootstrap::register_service(svc, 0x3000);

        assert!(job::get_job(label).is_some());
        assert!(bootstrap::look_up_service(svc).is_some());

        reset_all_state();

        assert!(job::get_job(label).is_none());
        assert!(bootstrap::look_up_service(svc).is_none());
    }
}
