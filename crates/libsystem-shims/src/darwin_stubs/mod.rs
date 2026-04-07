//! Stubs for macOS-specific APIs that have no Linux equivalent.
//!
//! Each sub-module provides no-op or safe-default implementations
//! so that guest Mach-O binaries can resolve these symbols on any
//! platform without crashing.

mod asl;
mod audit;
mod blocks_runtime;
mod cfnetwork;
mod common_crypto;
mod compression;
mod copyfile;
pub mod core_text;
mod disk_arbitration;
mod dispatch_concurrency;
mod dvt_foundation;
mod dispatch_data_source;
mod dispatch_extras;
pub(crate) mod dispatch_queue;
pub(crate) mod dyld_introspection;
mod iokit;
mod keymgr;
mod launch_services;
mod libedit;
mod launchd_stubs;
mod libxo_stubs;
mod linker_support;
pub(crate) mod mach_extras;
mod macho_arch;
mod malloc_zone;
mod misc;
mod mobile_gestalt;
mod notify;
mod os_log;
mod rosetta;
mod sandbox;
mod security;
mod spotlight;
mod system_configuration;
mod sysinfo;
mod xpc;
mod xpc_activity;
mod xpc_objects;
mod xpc_pipe;
mod xpc_transport;
mod swift_reflection;
mod swift_foundation_bridge;
mod swift_type_metadata;

use crate::table::ShimTable;

/// Install all darwin stub shims into the table.
pub fn install_darwin_stubs(table: &mut ShimTable) {
    asl::AslStubs::install(table);
    audit::AuditStubs::install(table);
    // blocks_runtime stubs are only needed on Linux; on macOS the real
    // libclosure.dylib is always available via StubResolver and must not
    // be shadowed (doing so breaks the Block copy/release ABI).
    #[cfg(not(target_os = "macos"))]
    blocks_runtime::BlocksRuntimeStubs::install(table);
    cfnetwork::CfNetworkStubs::install(table);
    common_crypto::CommonCryptoStubs::install(table);
    compression::CompressionStubs::install(table);
    copyfile::CopyfileStubs::install(table);
    core_text::CoreTextStubs::install(table);
    disk_arbitration::DiskArbitrationStubs::install(table);
    dvt_foundation::DvtFoundationStubs::install(table);
    dispatch_concurrency::DispatchConcurrencyStubs::install(table);
    dispatch_data_source::DispatchDataSourceStubs::install(table);
    dispatch_extras::DispatchExtrasStubs::install(table);
    iokit::IOKitStubs::install(table);
    launch_services::LaunchServicesStubs::install(table);
    launchd_stubs::LaunchdStubs::install(table);
    libedit::LibeditStubs::install(table);
    libxo_stubs::LibxoStubs::install(table);
    mach_extras::MachExtrasStubs::install(table);
    malloc_zone::MallocZoneStubs::install(table);
    misc::MiscStubs::install(table);
    mobile_gestalt::MobileGestaltStubs::install(table);
    notify::NotifyStubs::install(table);
    os_log::OsLogStubs::install(table);
    rosetta::RosettaStubs::install(table);
    sandbox::SandboxStubs::install(table);
    security::SecurityStubs::install(table);
    spotlight::SpotlightStubs::install(table);
    system_configuration::SystemConfigurationStubs::install(table);
    sysinfo::SysInfoStubs::install(table);
    dyld_introspection::DyldIntrospectionStubs::install(table);
    keymgr::KeymgrStubs::install(table);
    linker_support::LinkerSupportStubs::install(table);
    macho_arch::MachoArchStubs::install(table);
    xpc::XpcStubs::install(table);
    swift_reflection::LibswiftRemoteMirrorStubs::install(table);
    swift_foundation_bridge::SwiftFoundationBridgeStubs::install(table);
    swift_type_metadata::SwiftTypeMetadataStubs::install(table);
    log::debug!("darwin_stubs: registered all macOS-specific stubs");
}
