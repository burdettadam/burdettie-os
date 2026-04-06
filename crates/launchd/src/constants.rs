//! Apple `launch.h` / `launch_priv.h` string constants.
//!
//! These are the `#define` values from the open-source launchd headers.
//! They are used as dictionary keys in `launch_data_t` messages exchanged
//! between launchctl/daemons and launchd via `launch_msg()`.

// ── Top-level message keys (launch.h) ───────────────────────────────

pub const LAUNCH_KEY_SUBMITJOB: &str = "SubmitJob";
pub const LAUNCH_KEY_REMOVEJOB: &str = "RemoveJob";
pub const LAUNCH_KEY_STARTJOB: &str = "StartJob";
pub const LAUNCH_KEY_STOPJOB: &str = "StopJob";
pub const LAUNCH_KEY_GETJOB: &str = "GetJob";
pub const LAUNCH_KEY_GETJOBS: &str = "GetJobs";
pub const LAUNCH_KEY_CHECKIN: &str = "CheckIn";

// ── Private message keys (launch_priv.h) ────────────────────────────

pub const LAUNCH_KEY_SETUSERENVIRONMENT: &str = "SetUserEnvironment";
pub const LAUNCH_KEY_UNSETUSERENVIRONMENT: &str = "UnsetUserEnvironment";
pub const LAUNCH_KEY_SHUTDOWN: &str = "Shutdown";
pub const LAUNCH_KEY_SINGLEUSER: &str = "SingleUser";
pub const LAUNCH_KEY_GETRESOURCELIMITS: &str = "GetResourceLimits";
pub const LAUNCH_KEY_SETRESOURCELIMITS: &str = "SetResourceLimits";
pub const LAUNCH_KEY_GETRUSAGESELF: &str = "GetResourceUsageSelf";
pub const LAUNCH_KEY_GETRUSAGECHILDREN: &str = "GetResourceUsageChildren";
pub const LAUNCH_KEY_BATCHCONTROL: &str = "BatchControl";
pub const LAUNCH_KEY_BATCHQUERY: &str = "BatchQuery";

// ── Process type constants ──────────────────────────────────────────

pub const LAUNCH_KEY_PROCESSTYPE_APP: &str = "App";
pub const LAUNCH_KEY_PROCESSTYPE_STANDARD: &str = "Standard";
pub const LAUNCH_KEY_PROCESSTYPE_BACKGROUND: &str = "Background";
pub const LAUNCH_KEY_PROCESSTYPE_INTERACTIVE: &str = "Interactive";
pub const LAUNCH_KEY_PROCESSTYPE_ADAPTIVE: &str = "Adaptive";

// ── Job definition keys (launch.h LAUNCH_JOBKEY_*) ─────────────────

pub const LAUNCH_JOBKEY_DEFAULTS: &str = "__Defaults";
pub const LAUNCH_JOBKEY_LABEL: &str = "Label";
pub const LAUNCH_JOBKEY_DISABLED: &str = "Disabled";
pub const LAUNCH_JOBKEY_USERNAME: &str = "UserName";
pub const LAUNCH_JOBKEY_GROUPNAME: &str = "GroupName";
pub const LAUNCH_JOBKEY_TIMEOUT: &str = "TimeOut";
pub const LAUNCH_JOBKEY_EXITTIMEOUT: &str = "ExitTimeOut";
pub const LAUNCH_JOBKEY_INITGROUPS: &str = "InitGroups";
pub const LAUNCH_JOBKEY_SOCKETS: &str = "Sockets";
pub const LAUNCH_JOBKEY_MACHSERVICES: &str = "MachServices";
pub const LAUNCH_JOBKEY_MACHSERVICELOOKUPPOLICIES: &str = "MachServiceLookupPolicies";
pub const LAUNCH_JOBKEY_INETDCOMPATIBILITY: &str = "inetdCompatibility";
pub const LAUNCH_JOBKEY_ENABLEGLOBBING: &str = "EnableGlobbing";
pub const LAUNCH_JOBKEY_PROGRAMARGUMENTS: &str = "ProgramArguments";
pub const LAUNCH_JOBKEY_PROGRAM: &str = "Program";
pub const LAUNCH_JOBKEY_ONDEMAND: &str = "OnDemand";
pub const LAUNCH_JOBKEY_KEEPALIVE: &str = "KeepAlive";
pub const LAUNCH_JOBKEY_LIMITLOADTOHOSTS: &str = "LimitLoadToHosts";
pub const LAUNCH_JOBKEY_LIMITLOADFROMHOSTS: &str = "LimitLoadFromHosts";
pub const LAUNCH_JOBKEY_LIMITLOADTOSESSIONTYPE: &str = "LimitLoadToSessionType";
pub const LAUNCH_JOBKEY_LIMITLOADTOHARDWARE: &str = "LimitLoadToHardware";
pub const LAUNCH_JOBKEY_LIMITLOADFROMHARDWARE: &str = "LimitLoadFromHardware";
pub const LAUNCH_JOBKEY_RUNATLOAD: &str = "RunAtLoad";
pub const LAUNCH_JOBKEY_ROOTDIRECTORY: &str = "RootDirectory";
pub const LAUNCH_JOBKEY_WORKINGDIRECTORY: &str = "WorkingDirectory";
pub const LAUNCH_JOBKEY_ENVIRONMENTVARIABLES: &str = "EnvironmentVariables";
pub const LAUNCH_JOBKEY_USERENVIRONMENTVARIABLES: &str = "UserEnvironmentVariables";
pub const LAUNCH_JOBKEY_UMASK: &str = "Umask";
pub const LAUNCH_JOBKEY_NICE: &str = "Nice";
pub const LAUNCH_JOBKEY_HOPEFULLYEXITSFIRST: &str = "HopefullyExitsFirst";
pub const LAUNCH_JOBKEY_HOPEFULLYEXITSLAST: &str = "HopefullyExitsLast";
pub const LAUNCH_JOBKEY_LOWPRIORITYIO: &str = "LowPriorityIO";
pub const LAUNCH_JOBKEY_SESSIONCREATE: &str = "SessionCreate";
pub const LAUNCH_JOBKEY_STARTONMOUNT: &str = "StartOnMount";
pub const LAUNCH_JOBKEY_SOFTRESOURCELIMITS: &str = "SoftResourceLimits";
pub const LAUNCH_JOBKEY_HARDRESOURCELIMITS: &str = "HardResourceLimits";
pub const LAUNCH_JOBKEY_STANDARDINPATH: &str = "StandardInPath";
pub const LAUNCH_JOBKEY_STANDARDOUTPATH: &str = "StandardOutPath";
pub const LAUNCH_JOBKEY_STANDARDERRORPATH: &str = "StandardErrorPath";
pub const LAUNCH_JOBKEY_DEBUG: &str = "Debug";
pub const LAUNCH_JOBKEY_WAITFORDEBUGGER: &str = "WaitForDebugger";
pub const LAUNCH_JOBKEY_QUEUEDIRECTORIES: &str = "QueueDirectories";
pub const LAUNCH_JOBKEY_WATCHPATHS: &str = "WatchPaths";
pub const LAUNCH_JOBKEY_STARTINTERVAL: &str = "StartInterval";
pub const LAUNCH_JOBKEY_STARTCALENDARINTERVAL: &str = "StartCalendarInterval";
pub const LAUNCH_JOBKEY_BONJOURFDS: &str = "BonjourFDs";
pub const LAUNCH_JOBKEY_LASTEXITSTATUS: &str = "LastExitStatus";
pub const LAUNCH_JOBKEY_PID: &str = "PID";
pub const LAUNCH_JOBKEY_THROTTLEINTERVAL: &str = "ThrottleInterval";
pub const LAUNCH_JOBKEY_LAUNCHONLYONCE: &str = "LaunchOnlyOnce";
pub const LAUNCH_JOBKEY_ABANDONPROCESSGROUP: &str = "AbandonProcessGroup";
pub const LAUNCH_JOBKEY_IGNOREPROCESSGROUPATSHUTDOWN: &str = "IgnoreProcessGroupAtShutdown";
pub const LAUNCH_JOBKEY_POLICIES: &str = "Policies";
pub const LAUNCH_JOBKEY_ENABLETRANSACTIONS: &str = "EnableTransactions";
pub const LAUNCH_JOBKEY_CFBUNDLEIDENTIFIER: &str = "CFBundleIdentifier";
pub const LAUNCH_JOBKEY_PROCESSTYPE: &str = "ProcessType";
pub const LAUNCH_JOBKEY_LAUNCHEVENTS: &str = "LaunchEvents";

// ── Policy keys ─────────────────────────────────────────────────────

pub const LAUNCH_JOBPOLICY_DENYCREATINGOTHERJOBS: &str = "DenyCreatingOtherJobs";

// ── inetd compatibility ─────────────────────────────────────────────

pub const LAUNCH_JOBINETDCOMPATIBILITY_WAIT: &str = "Wait";

// ── MachServices sub-keys ───────────────────────────────────────────

pub const LAUNCH_JOBKEY_MACH_RESETATCLOSE: &str = "ResetAtClose";
pub const LAUNCH_JOBKEY_MACH_HIDEUNTILCHECKIN: &str = "HideUntilCheckIn";
pub const LAUNCH_JOBKEY_MACH_DRAINMESSAGESONCRASH: &str = "DrainMessagesOnCrash";
pub const LAUNCH_JOBKEY_MACH_PINGEVENTUPDATES: &str = "PingEventUpdates";

// ── KeepAlive sub-keys ──────────────────────────────────────────────

pub const LAUNCH_JOBKEY_KEEPALIVE_SUCCESSFULEXIT: &str = "SuccessfulExit";
pub const LAUNCH_JOBKEY_KEEPALIVE_NETWORKSTATE: &str = "NetworkState";
pub const LAUNCH_JOBKEY_KEEPALIVE_PATHSTATE: &str = "PathState";
pub const LAUNCH_JOBKEY_KEEPALIVE_OTHERJOBACTIVE: &str = "OtherJobActive";
pub const LAUNCH_JOBKEY_KEEPALIVE_OTHERJOBENABLED: &str = "OtherJobEnabled";
pub const LAUNCH_JOBKEY_KEEPALIVE_AFTERINITIALDEMAND: &str = "AfterInitialDemand";
pub const LAUNCH_JOBKEY_KEEPALIVE_CRASHED: &str = "Crashed";

// ── Calendar interval sub-keys ──────────────────────────────────────

pub const LAUNCH_JOBKEY_CAL_MINUTE: &str = "Minute";
pub const LAUNCH_JOBKEY_CAL_HOUR: &str = "Hour";
pub const LAUNCH_JOBKEY_CAL_DAY: &str = "Day";
pub const LAUNCH_JOBKEY_CAL_WEEKDAY: &str = "Weekday";
pub const LAUNCH_JOBKEY_CAL_MONTH: &str = "Month";

// ── Resource limit sub-keys ─────────────────────────────────────────

pub const LAUNCH_JOBKEY_RESOURCELIMIT_CORE: &str = "Core";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_CPU: &str = "CPU";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_DATA: &str = "Data";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_FSIZE: &str = "FileSize";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_MEMLOCK: &str = "MemoryLock";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_NOFILE: &str = "NumberOfFiles";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_NPROC: &str = "NumberOfProcesses";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_RSS: &str = "ResidentSetSize";
pub const LAUNCH_JOBKEY_RESOURCELIMIT_STACK: &str = "Stack";

// ── Disabled sub-keys ───────────────────────────────────────────────

pub const LAUNCH_JOBKEY_DISABLED_MACHINETYPE: &str = "MachineType";
pub const LAUNCH_JOBKEY_DISABLED_MODELNAME: &str = "ModelName";

// ── Socket sub-keys ─────────────────────────────────────────────────

pub const LAUNCH_JOBSOCKETKEY_TYPE: &str = "SockType";
pub const LAUNCH_JOBSOCKETKEY_PASSIVE: &str = "SockPassive";
pub const LAUNCH_JOBSOCKETKEY_BONJOUR: &str = "Bonjour";
pub const LAUNCH_JOBSOCKETKEY_SECUREWITHKEY: &str = "SecureSocketWithKey";
pub const LAUNCH_JOBSOCKETKEY_PATHNAME: &str = "SockPathName";
pub const LAUNCH_JOBSOCKETKEY_PATHMODE: &str = "SockPathMode";
pub const LAUNCH_JOBSOCKETKEY_NODENAME: &str = "SockNodeName";
pub const LAUNCH_JOBSOCKETKEY_SERVICENAME: &str = "SockServiceName";
pub const LAUNCH_JOBSOCKETKEY_FAMILY: &str = "SockFamily";
pub const LAUNCH_JOBSOCKETKEY_PROTOCOL: &str = "SockProtocol";
pub const LAUNCH_JOBSOCKETKEY_MULTICASTGROUP: &str = "MulticastGroup";

// ── Private job keys (launch_priv.h) ────────────────────────────────

pub const LAUNCH_JOBKEY_TRANSACTIONCOUNT: &str = "TransactionCount";
pub const LAUNCH_JOBKEY_QUARANTINEDATA: &str = "QuarantineData";
pub const LAUNCH_JOBKEY_SANDBOXPROFILE: &str = "SandboxProfile";
pub const LAUNCH_JOBKEY_SANDBOXFLAGS: &str = "SandboxFlags";
pub const LAUNCH_JOBKEY_SANDBOX_NAMED: &str = "Named";
pub const LAUNCH_JOBKEY_SANDBOXCONTAINER: &str = "SandboxContainer";
pub const LAUNCH_JOBKEY_JETSAMPROPERTIES: &str = "JetsamProperties";
pub const LAUNCH_JOBKEY_JETSAMPRIORITY: &str = "JetsamPriority";
pub const LAUNCH_JOBKEY_JETSAMMEMORYLIMIT: &str = "JetsamMemoryLimit";
pub const LAUNCH_JOBKEY_SECURITYSESSIONUUID: &str = "SecuritySessionUUID";
pub const LAUNCH_JOBKEY_DISABLEASLR: &str = "DisableASLR";
pub const LAUNCH_JOBKEY_XPCDOMAIN: &str = "XPCDomain";
pub const LAUNCH_JOBKEY_POSIXSPAWNTYPE: &str = "POSIXSpawnType";
pub const LAUNCH_JOBKEY_PERJOBMACHSERVICES: &str = "PerJobMachServices";
pub const LAUNCH_JOBKEY_SERVICEIPC: &str = "ServiceIPC";
pub const LAUNCH_JOBKEY_BINARYORDERPREFERENCE: &str = "BinaryOrderPreference";
pub const LAUNCH_JOBKEY_MACHEXCEPTIONHANDLER: &str = "MachExceptionHandler";
pub const LAUNCH_JOBKEY_MULTIPLEINSTANCES: &str = "MultipleInstances";
pub const LAUNCH_JOBKEY_EVENTMONITOR: &str = "EventMonitor";
pub const LAUNCH_JOBKEY_SHUTDOWNMONITOR: &str = "ShutdownMonitor";
pub const LAUNCH_JOBKEY_BEGINTRANSACTIONATSHUTDOWN: &str = "BeginTransactionAtShutdown";
pub const LAUNCH_JOBKEY_XPCDOMAINBOOTSTRAPPER: &str = "XPCDomainBootstrapper";
pub const LAUNCH_JOBKEY_ASID: &str = "AuditSessionID";
pub const LAUNCH_JOBKEY_JOINGUISESSION: &str = "JoinGUISession";
pub const LAUNCH_JOBKEY_EMBEDDEDPRIVILEGEDISPENSATION: &str = "EmbeddedPrivilegeDispensation";

// ── Mach private sub-keys ───────────────────────────────────────────

pub const LAUNCH_JOBKEY_MACH_KUNCSERVER: &str = "kUNCServer";
pub const LAUNCH_JOBKEY_MACH_EXCEPTIONSERVER: &str = "ExceptionServer";
pub const LAUNCH_JOBKEY_MACH_TASKSPECIALPORT: &str = "TaskSpecialPort";
pub const LAUNCH_JOBKEY_MACH_HOSTSPECIALPORT: &str = "HostSpecialPort";
pub const LAUNCH_JOBKEY_MACH_ENTERKERNELDEBUGGERONCLOSE: &str = "EnterKernelDebuggerOnClose";

// ── Environment / socket private ────────────────────────────────────

pub const LAUNCHD_SOCKET_ENV: &str = "LAUNCHD_SOCKET";
pub const LAUNCHD_TRUSTED_FD_ENV: &str = "__LAUNCHD_FD";
pub const LAUNCHD_ASYNC_MSG_KEY: &str = "_AsyncMessage";

// ── Bootstrap constants (bootstrap.h) ───────────────────────────────

pub const BOOTSTRAP_MAX_NAME_LEN: usize = 128;
pub const BOOTSTRAP_MAX_CMD_LEN: usize = 512;
pub const BOOTSTRAP_MAX_LOOKUP_COUNT: usize = 20;

pub const BOOTSTRAP_SUCCESS: u32 = 0;
pub const BOOTSTRAP_NOT_PRIVILEGED: u32 = 1100;
pub const BOOTSTRAP_NAME_IN_USE: u32 = 1101;
pub const BOOTSTRAP_UNKNOWN_SERVICE: u32 = 1102;
pub const BOOTSTRAP_SERVICE_ACTIVE: u32 = 1103;
pub const BOOTSTRAP_BAD_COUNT: u32 = 1104;
pub const BOOTSTRAP_NO_MEMORY: u32 = 1105;
pub const BOOTSTRAP_NO_CHILDREN: u32 = 1106;

pub const BOOTSTRAP_STATUS_INACTIVE: u32 = 0;
pub const BOOTSTRAP_STATUS_ACTIVE: u32 = 1;
pub const BOOTSTRAP_STATUS_ON_DEMAND: u32 = 2;

// ── Bootstrap private flags (bootstrap_priv.h) ──────────────────────

pub const BOOTSTRAP_PER_PID_SERVICE: u64 = 1 << 0;
pub const BOOTSTRAP_ALLOW_LOOKUP: u64 = 1 << 1;
pub const BOOTSTRAP_DENY_JOB_CREATION: u64 = 1 << 2;
pub const BOOTSTRAP_PRIVILEGED_SERVER: u64 = 1 << 3;
pub const BOOTSTRAP_FORCE_LOCAL: u64 = 1 << 4;
pub const BOOTSTRAP_SPECIFIC_INSTANCE: u64 = 1 << 5;
pub const BOOTSTRAP_STRICT_CHECKIN: u64 = 1 << 6;
pub const BOOTSTRAP_STRICT_LOOKUP: u64 = 1 << 7;

// ── vproc GSK keys (vproc_priv.h) ──────────────────────────────────

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VprocGsk {
    Zero = 0,
    LastExitStatus,
    GlobalOnDemand,
    MgrUid,
    MgrPid,
    IsManaged,
    MgrName,
    BasicKeepalive,
    StartInterval,
    IdleTimeout,
    ExitTimeout,
    Environment,
    AllJobs,
    GlobalLogMask,
    GlobalUmask,
    AbandonProcessGroup,
    TransactionsEnabled,
    WeirdBootstrap,
    WaitForDebugger,
    SecuritySession,
    ShutdownDebugging,
    VerboseBoot,
    PeruserSuspend,
    PeruserResume,
    JobOverridesDb,
    JobCacheDb,
    EmbeddedRootEquivalent,
}

// ── vproc session names (vproc_priv.h) ──────────────────────────────

pub const VPROCMGR_SESSION_LOGINWINDOW: &str = "LoginWindow";
pub const VPROCMGR_SESSION_BACKGROUND: &str = "Background";
pub const VPROCMGR_SESSION_AQUA: &str = "Aqua";
pub const VPROCMGR_SESSION_STANDARDIO: &str = "StandardIO";
pub const VPROCMGR_SESSION_SYSTEM: &str = "System";

// ── Spawn flags (launch_priv.h) ─────────────────────────────────────

pub const SPAWN_VIA_LAUNCHD_STOPPED: u64 = 0x0001;
pub const SPAWN_VIA_LAUNCHD_TALAPP: u64 = 0x0002;
pub const SPAWN_VIA_LAUNCHD_WIDGET: u64 = 0x0004;
pub const SPAWN_VIA_LAUNCHD_DISABLE_ASLR: u64 = 0x0008;

// ── Vproc magic values ──────────────────────────────────────────────

pub const VPROC_MAGIC_UNLOAD_SIGNAL: u32 = 0x4141_504C;
pub const VPROCFLAG_STALL_JOB_EXEC: u32 = 1 << 1;

// ── Jetsam property keys ────────────────────────────────────────────

pub const JETSAM_PROPERTY_PRIORITY: &str = "Priority";
pub const JETSAM_PROPERTY_MEMORYLIMIT: &str = "MemoryLimitMB";
