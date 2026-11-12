//! Data symbol store — provides stable non-NULL addresses for imported
//! data symbols.
//!
//! Guest Mach-O binaries that import data symbols (constants, type
//! tokens, IVAR offsets) need to read valid data from the resolved
//! address.  Unlike function symbols where the bind trap returning 0
//! can be checked by callers, a data symbol resolving to 0 causes an
//! immediate NULL-dereference when the guest loads from it.
//!
//! All data here is 'static — allocated once, lives for process lifetime.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Process-lifetime singleton for the data symbol store.
static GLOBAL_STORE: OnceLock<DataSymbolStore> = OnceLock::new();

/// Provides stable non-NULL addresses for data symbols.
///
/// Every entry maps a symbol name (with leading underscore, as it
/// appears in the Mach-O import table) to a `u64` address of a
/// statically allocated value.
pub struct DataSymbolStore {
    symbols: HashMap<&'static str, u64>,
}

impl DataSymbolStore {
    /// Return the process-wide singleton, initializing on first call.
    ///
    /// All data symbols are `'static` addresses (statics or dlsym results)
    /// so the store is safe to reuse across pipeline runs.
    pub fn global() -> &'static Self {
        GLOBAL_STORE.get_or_init(Self::new)
    }

    /// Create a new store with all known data symbol registrations.
    pub fn new() -> Self {
        let mut store = Self {
            symbols: HashMap::with_capacity(128),
        };
        store.register_xpc_types();
        store.register_xpc_errors();
        store.register_dispatch_source_types();
        store.register_ndr_record();
        store.register_sys_signame();
        store.register_cf_version_keys();
        store.register_foundation_constants();
        store.register_block_globals();
        store.register_disk_arbitration_keys();
        store.register_launch_services_keys();
        store.register_ns_constants();
        store.register_nstat_keys();
        store.register_swift_runtime_globals();
        store.register_objc_runtime_globals();
        store.register_objc_ivar_offsets();
        store.register_dvt_ide_constants();
        store
    }

    /// Resolve a data symbol name to its address.
    pub fn resolve(&self, name: &str) -> Option<u64> {
        self.symbols.get(name).copied()
    }

    /// Number of registered data symbols.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    // ── XPC type constants ──────────────────────────────────────

    fn register_xpc_types(&mut self) {
        // Each XPC type constant is an opaque pointer used only for
        // identity comparison (xpc_get_type(obj) == XPC_TYPE_DICTIONARY).
        // We allocate unique 8-byte tokens — the value doesn't matter,
        // only that each address is unique and non-NULL.
        static XPC_TYPE_ACTIVITY: u64 = 0x5850_4301;
        static XPC_TYPE_ARRAY: u64 = 0x5850_4302;
        static XPC_TYPE_BOOL: u64 = 0x5850_4303;
        static XPC_TYPE_CONNECTION: u64 = 0x5850_4304;
        static XPC_TYPE_DATA: u64 = 0x5850_4305;
        static XPC_TYPE_DICTIONARY: u64 = 0x5850_4306;
        static XPC_TYPE_ERROR: u64 = 0x5850_4307;
        static XPC_TYPE_STRING: u64 = 0x5850_4308;
        static XPC_TYPE_INT64: u64 = 0x5850_4309;
        static XPC_TYPE_UINT64: u64 = 0x5850_430A;
        static XPC_TYPE_DATE: u64 = 0x5850_430B;
        static XPC_TYPE_FD: u64 = 0x5850_430C;
        static XPC_TYPE_NULL: u64 = 0x5850_430D;
        static XPC_TYPE_SHMEM: u64 = 0x5850_430E;
        static XPC_TYPE_UUID: u64 = 0x5850_430F;
        static XPC_TYPE_ENDPOINT: u64 = 0x5850_4310;

        self.symbols.insert(
            "__xpc_type_activity",
            &XPC_TYPE_ACTIVITY as *const u64 as u64,
        );
        self.symbols
            .insert("__xpc_type_array", &XPC_TYPE_ARRAY as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_bool", &XPC_TYPE_BOOL as *const u64 as u64);
        self.symbols.insert(
            "__xpc_type_connection",
            &XPC_TYPE_CONNECTION as *const u64 as u64,
        );
        self.symbols
            .insert("__xpc_type_data", &XPC_TYPE_DATA as *const u64 as u64);
        self.symbols.insert(
            "__xpc_type_dictionary",
            &XPC_TYPE_DICTIONARY as *const u64 as u64,
        );
        self.symbols
            .insert("__xpc_type_error", &XPC_TYPE_ERROR as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_string", &XPC_TYPE_STRING as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_int64", &XPC_TYPE_INT64 as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_uint64", &XPC_TYPE_UINT64 as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_date", &XPC_TYPE_DATE as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_fd", &XPC_TYPE_FD as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_null", &XPC_TYPE_NULL as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_shmem", &XPC_TYPE_SHMEM as *const u64 as u64);
        self.symbols
            .insert("__xpc_type_uuid", &XPC_TYPE_UUID as *const u64 as u64);
        self.symbols.insert(
            "__xpc_type_endpoint",
            &XPC_TYPE_ENDPOINT as *const u64 as u64,
        );
    }

    // ── XPC error constants ─────────────────────────────────────

    fn register_xpc_errors(&mut self) {
        // XPC error constants are xpc_object_t pointers. Code checks
        // pointer identity (error == XPC_ERROR_CONNECTION_INTERRUPTED).
        // We allocate unique sentinel values.
        static XPC_ERR_INTERRUPTED: u64 = 0x5850_E001;
        static XPC_ERR_INVALID: u64 = 0x5850_E002;
        static XPC_ERR_TERMINATION: u64 = 0x5850_E003;
        static XPC_ERR_KEY_DESC: u64 = 0x5850_E004;
        static XPC_ERR_PEER_SIGNING: u64 = 0x5850_E005;

        self.symbols.insert(
            "__xpc_error_connection_interrupted",
            &XPC_ERR_INTERRUPTED as *const u64 as u64,
        );
        self.symbols.insert(
            "__xpc_error_connection_invalid",
            &XPC_ERR_INVALID as *const u64 as u64,
        );
        self.symbols.insert(
            "__xpc_error_termination_imminent",
            &XPC_ERR_TERMINATION as *const u64 as u64,
        );
        self.symbols.insert(
            "__xpc_error_key_description",
            &XPC_ERR_KEY_DESC as *const u64 as u64,
        );
        self.symbols.insert(
            "__xpc_error_peer_code_signing_requirement",
            &XPC_ERR_PEER_SIGNING as *const u64 as u64,
        );
    }

    // ── dispatch source types ───────────────────────────────────

    fn register_dispatch_source_types(&mut self) {
        // dispatch_source_type_t constants — opaque pointers used to
        // create dispatch sources. Identity comparison only.
        static DST_DATA_ADD: u64 = 0x4453_5401;
        static DST_DATA_OR: u64 = 0x4453_5402;
        static DST_DATA_REPLACE: u64 = 0x4453_5403;
        static DST_MACH_SEND: u64 = 0x4453_5404;
        static DST_MEMORYPRESSURE: u64 = 0x4453_5405;
        static DST_TIMER: u64 = 0x4453_5406;
        static DST_READ: u64 = 0x4453_5407;
        static DST_WRITE: u64 = 0x4453_5408;
        static DST_PROC: u64 = 0x4453_5409;
        static DST_SIGNAL: u64 = 0x4453_540A;
        static DST_VNODE: u64 = 0x4453_540B;
        static DST_MACH_RECV: u64 = 0x4453_540C;

        self.symbols.insert(
            "__dispatch_source_type_data_add",
            &DST_DATA_ADD as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_data_or",
            &DST_DATA_OR as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_data_replace",
            &DST_DATA_REPLACE as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_mach_send",
            &DST_MACH_SEND as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_memorypressure",
            &DST_MEMORYPRESSURE as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_timer",
            &DST_TIMER as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_read",
            &DST_READ as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_write",
            &DST_WRITE as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_proc",
            &DST_PROC as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_signal",
            &DST_SIGNAL as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_vnode",
            &DST_VNODE as *const u64 as u64,
        );
        self.symbols.insert(
            "__dispatch_source_type_mach_recv",
            &DST_MACH_RECV as *const u64 as u64,
        );
    }

    // ── NDR_record ──────────────────────────────────────────────

    fn register_ndr_record(&mut self) {
        // NDR_record is the Network Data Representation record used by
        // Mach MIG. It describes the byte order and encoding of the
        // local machine.
        //
        // Layout (8 bytes):
        //   mig_vers:      u8 = 0
        //   if_vers:       u8 = 0
        //   reserved1:     u8 = 0
        //   mig_encoding:  u8 = 1  (NDR_PROTOCOL_2_0)
        //   int_rep:       u8 = 1  (NDR_LITTLE_ENDIAN)
        //   char_rep:      u8 = 0  (NDR_ASCII)
        //   float_rep:     u8 = 0  (NDR_IEEE_FLOAT)
        //   reserved2:     u8 = 0
        #[repr(C)]
        struct NdrRecord {
            mig_vers: u8,
            if_vers: u8,
            reserved1: u8,
            mig_encoding: u8,
            int_rep: u8,
            char_rep: u8,
            float_rep: u8,
            reserved2: u8,
        }

        static NDR_RECORD: NdrRecord = NdrRecord {
            mig_vers: 0,
            if_vers: 0,
            reserved1: 0,
            mig_encoding: 1,
            int_rep: 1,
            char_rep: 0,
            float_rep: 0,
            reserved2: 0,
        };

        self.symbols
            .insert("_NDR_record", &NDR_RECORD as *const NdrRecord as u64);
    }

    // ── sys_signame ─────────────────────────────────────────────

    fn register_sys_signame(&mut self) {
        // sys_signame is a table of signal name strings indexed by
        // signal number. On macOS it's exported from libSystem.
        static SIG_NAMES: [&[u8]; 32] = [
            b"EXIT\0",
            b"HUP\0",
            b"INT\0",
            b"QUIT\0",
            b"ILL\0",
            b"TRAP\0",
            b"ABRT\0",
            b"EMT\0",
            b"FPE\0",
            b"KILL\0",
            b"BUS\0",
            b"SEGV\0",
            b"SYS\0",
            b"PIPE\0",
            b"ALRM\0",
            b"TERM\0",
            b"URG\0",
            b"STOP\0",
            b"TSTP\0",
            b"CONT\0",
            b"CHLD\0",
            b"TTIN\0",
            b"TTOU\0",
            b"IO\0",
            b"XCPU\0",
            b"XFSZ\0",
            b"VTALRM\0",
            b"PROF\0",
            b"WINCH\0",
            b"INFO\0",
            b"USR1\0",
            b"USR2\0",
        ];

        // Build a static array of pointers to the signal name strings.
        // This is what sys_signame points to on macOS.
        static SIG_NAME_PTRS: std::sync::OnceLock<[u64; 32]> = std::sync::OnceLock::new();

        let ptrs = SIG_NAME_PTRS.get_or_init(|| {
            let mut arr = [0u64; 32];
            for (i, name) in SIG_NAMES.iter().enumerate() {
                arr[i] = name.as_ptr() as u64;
            }
            arr
        });

        self.symbols.insert("_sys_signame", ptrs.as_ptr() as u64);
    }

    // ── CF system version keys ──────────────────────────────────

    fn register_cf_version_keys(&mut self) {
        // These are CFStringRef constants — fallback for when the cf-lite
        // ShimTable entries aren't present.  The ShimTable provides proper
        // CFString-backed addresses; these sentinels just prevent NULL deref.
        // The symbol names must include the "Key" suffix to match real imports.
        static PRODUCT_NAME: u64 = 0x4346_5601;
        static PRODUCT_VERSION: u64 = 0x4346_5602;
        static BUILD_VERSION: u64 = 0x4346_5603;
        static PRODUCT_VERSION_EXTRA: u64 = 0x4346_5604;

        self.symbols.insert(
            "__kCFSystemVersionProductNameKey",
            &PRODUCT_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "__kCFSystemVersionProductVersionKey",
            &PRODUCT_VERSION as *const u64 as u64,
        );
        self.symbols.insert(
            "__kCFSystemVersionBuildVersionKey",
            &BUILD_VERSION as *const u64 as u64,
        );
        self.symbols.insert(
            "__kCFSystemVersionProductVersionExtraKey",
            &PRODUCT_VERSION_EXTRA as *const u64 as u64,
        );
    }

    // ── Foundation / CF constants ────────────────────────────────

    fn register_foundation_constants(&mut self) {
        // CFBoolean singletons — used for identity comparison in CF/ObjC code.
        // The guest reads the pointer from the GOT; well-formed code only
        // compares it for identity (ptr == kCFBooleanTrue), so any distinct
        // non-NULL address works.
        static CF_BOOLEAN_TRUE: u64 = 0x4346_B001;
        static CF_BOOLEAN_FALSE: u64 = 0x4346_B000;

        // CFConstantStringClassReference — the ISA pointer embedded in all
        // __TEXT,__cstring_objects. The guest should never chase this on
        // a live basis; it's just used as a sentinel by the ObjC runtime.
        static CF_CONST_STR_CLASS: u64 = 0x4346_5300;

        self.symbols
            .insert("___kCFBooleanTrue", &CF_BOOLEAN_TRUE as *const u64 as u64);
        self.symbols
            .insert("___kCFBooleanFalse", &CF_BOOLEAN_FALSE as *const u64 as u64);
        self.symbols.insert(
            "___CFConstantStringClassReference",
            &CF_CONST_STR_CLASS as *const u64 as u64,
        );

        // kCFRunLoop modes — CFStringRef constants used as mode identifiers.
        // Non-NULL opaque addresses are sufficient for mode dispatch.
        static CFRL_DEFAULT: u64 = 0x4346_4D01;
        static CFRL_COMMON: u64 = 0x4346_4D02;
        static NSDEFAULT_MODE: u64 = 0x4E53_4D01;
        static NSCOMMON_MODES: u64 = 0x4E53_4D02;
        static NSEVENT_TRACK: u64 = 0x4E53_4D03;
        static NSMODAL_PANEL: u64 = 0x4E53_4D04;
        static NSCONN_REPLY: u64 = 0x4E53_4D05;

        self.symbols
            .insert("_kCFRunLoopDefaultMode", &CFRL_DEFAULT as *const u64 as u64);
        self.symbols
            .insert("_kCFRunLoopCommonModes", &CFRL_COMMON as *const u64 as u64);
        self.symbols.insert(
            "_NSDefaultRunLoopMode",
            &NSDEFAULT_MODE as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSRunLoopCommonModes",
            &NSCOMMON_MODES as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSEventTrackingRunLoopMode",
            &NSEVENT_TRACK as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSModalPanelRunLoopMode",
            &NSMODAL_PANEL as *const u64 as u64,
        );
        self.symbols
            .insert("_NSConnectionReplyMode", &NSCONN_REPLY as *const u64 as u64);
    }

    // ── Blocks runtime globals ────────────────────────────────────

    fn register_block_globals(&mut self) {
        // __NSConcreteGlobalBlock / __NSConcreteStackBlock — ISA pointers
        // embedded by the Clang blocks runtime in block descriptor structs.
        // The guest's _Block_copy/_Block_release calls inspect these for the
        // block kind (global = no copy needed, stack = heap-copy needed).
        // Our blocks_runtime shim already handles the copy/release paths;
        // these sentinels just prevent NULL-dereference on ISA reads.
        static NS_GLOBAL_BLOCK: u64 = 0x424C_4B01;
        static NS_STACK_BLOCK: u64 = 0x424C_4B02;
        static NS_MALLOC_BLOCK: u64 = 0x424C_4B03;
        static NS_AUTO_BLOCK: u64 = 0x424C_4B04;

        self.symbols.insert(
            "___NSConcreteGlobalBlock",
            &NS_GLOBAL_BLOCK as *const u64 as u64,
        );
        self.symbols.insert(
            "___NSConcreteStackBlock",
            &NS_STACK_BLOCK as *const u64 as u64,
        );
        self.symbols.insert(
            "___NSConcreteMallocBlock",
            &NS_MALLOC_BLOCK as *const u64 as u64,
        );
        self.symbols.insert(
            "___NSConcreteAutoBlock",
            &NS_AUTO_BLOCK as *const u64 as u64,
        );
        // libdispatch block globals
        self.symbols.insert(
            "__dispatch_block_create_with_qos_class",
            &NS_GLOBAL_BLOCK as *const u64 as u64,
        );
    }

    // ── DiskArbitration key constants ─────────────────────────────

    fn register_disk_arbitration_keys(&mut self) {
        // DADisk/DAMedia description dictionary keys — CFStringRef constants.
        // Used as dictionary keys; opaque non-NULL addresses are sufficient.
        static DA_MEDIA_NAME: u64 = 0x4441_4B01;
        static DA_VOLUME_NAME: u64 = 0x4441_4B02;
        static DA_VOLUME_PATH: u64 = 0x4441_4B03;
        static DA_VOLUME_KIND: u64 = 0x4441_4B04;
        static DA_MEDIA_KIND: u64 = 0x4441_4B05;
        static DA_MEDIA_WRITABLE: u64 = 0x4441_4B06;
        static DA_MEDIA_EJECTABLE: u64 = 0x4441_4B07;
        static DA_MEDIA_WHOLE: u64 = 0x4441_4B08;
        static DA_MEDIA_LEAF: u64 = 0x4441_4B09;
        static DA_MEDIA_SIZE: u64 = 0x4441_4B0A;
        static DA_BUS_NAME: u64 = 0x4441_4B0B;
        static DA_BUS_PATH: u64 = 0x4441_4B0C;
        static DA_DEVICE_MODEL: u64 = 0x4441_4B0D;
        static DA_DEVICE_PATH: u64 = 0x4441_4B0E;
        static DA_DEVICE_PROTO: u64 = 0x4441_4B0F;

        self.symbols.insert(
            "_kDADiskDescriptionMediaNameKey",
            &DA_MEDIA_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionVolumeNameKey",
            &DA_VOLUME_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionVolumePathKey",
            &DA_VOLUME_PATH as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionVolumeKindKey",
            &DA_VOLUME_KIND as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionMediaKindKey",
            &DA_MEDIA_KIND as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionMediaWritableKey",
            &DA_MEDIA_WRITABLE as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionMediaEjectableKey",
            &DA_MEDIA_EJECTABLE as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionMediaWholeKey",
            &DA_MEDIA_WHOLE as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionMediaLeafKey",
            &DA_MEDIA_LEAF as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionMediaSizeKey",
            &DA_MEDIA_SIZE as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionBusNameKey",
            &DA_BUS_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionBusPathKey",
            &DA_BUS_PATH as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionDeviceModelKey",
            &DA_DEVICE_MODEL as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionDevicePathKey",
            &DA_DEVICE_PATH as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDADiskDescriptionDeviceProtocolKey",
            &DA_DEVICE_PROTO as *const u64 as u64,
        );
    }

    // ── LaunchServices key constants ──────────────────────────────

    fn register_launch_services_keys(&mut self) {
        // LSItem/Application attribute string keys — used as dictionary keys
        // in launch services queries. Opaque non-NULL sentinels are sufficient.
        static LS_APP_CAT: u64 = 0x4C53_4B01;
        static LS_CONTENT_TYPE: u64 = 0x4C53_4B02;
        static LS_ROLE: u64 = 0x4C53_4B03;
        static LS_DISPLAY_NAME: u64 = 0x4C53_4B04;
        static LS_PATH: u64 = 0x4C53_4B05;
        static LS_BUNDLE_ID: u64 = 0x4C53_4B06;
        static LS_VERSION: u64 = 0x4C53_4B07;
        static LS_KIND_STR: u64 = 0x4C53_4B08;
        static LS_REQ_OS: u64 = 0x4C53_4B09;
        static LS_PKG_TYPE: u64 = 0x4C53_4B0A;
        static LS_PKG_CREATOR: u64 = 0x4C53_4B0B;
        // LaunchServices error domain
        static LS_ERR_DOMAIN: u64 = 0x4C53_4B0C;

        self.symbols.insert(
            "_kLSApplicationCategoryTypeKey",
            &LS_APP_CAT as *const u64 as u64,
        );
        self.symbols
            .insert("_kLSItemContentType", &LS_CONTENT_TYPE as *const u64 as u64);
        self.symbols
            .insert("_kLSItemRoles", &LS_ROLE as *const u64 as u64);
        self.symbols
            .insert("_kLSItemDisplayName", &LS_DISPLAY_NAME as *const u64 as u64);
        self.symbols
            .insert("_kLSItemPath", &LS_PATH as *const u64 as u64);
        self.symbols
            .insert("_kLSItemBundle", &LS_BUNDLE_ID as *const u64 as u64);
        self.symbols
            .insert("_kLSItemVersion", &LS_VERSION as *const u64 as u64);
        self.symbols
            .insert("_kLSItemKindString", &LS_KIND_STR as *const u64 as u64);
        self.symbols
            .insert("_kLSItemRequiredOSVersion", &LS_REQ_OS as *const u64 as u64);
        self.symbols
            .insert("_kLSItemPackageType", &LS_PKG_TYPE as *const u64 as u64);
        self.symbols.insert(
            "_kLSItemPackageCreator",
            &LS_PKG_CREATOR as *const u64 as u64,
        );
        self.symbols.insert(
            "_kLSApplicationLaunchErrorDomain",
            &LS_ERR_DOMAIN as *const u64 as u64,
        );
    }

    // ── NSError / Foundation key constants ────────────────────────

    fn register_ns_constants(&mut self) {
        // NSError user-info key strings. The guest stores these in an
        // NSError user-info dictionary; they're used as dictionary keys
        // (pointer identity or string comparison). Non-NULL opaque addresses
        // prevent NULL-dereference during dictionary construction.
        static NS_LOC_DESC: u64 = 0x4E53_4501;
        static NS_LOC_REASON: u64 = 0x4E53_4502;
        static NS_LOC_REC: u64 = 0x4E53_4503;
        static NS_LOC_FAIL: u64 = 0x4E53_4504;
        static NS_UNDERLYING: u64 = 0x4E53_4505;
        static NS_URL_ERR: u64 = 0x4E53_4506;
        static NS_FILE_PATH: u64 = 0x4E53_4507;
        static NS_STR_ERR: u64 = 0x4E53_4508;
        static NS_DEBUGDESC: u64 = 0x4E53_4509;
        // Notification names
        static NS_PROC_INFO: u64 = 0x4E53_4510;
        #[allow(dead_code)]
        static NS_LOW_DISK: u64 = 0x4E53_4511;
        // NSBundle/NSApplication keys
        static NS_BUNDLE_DID_LOAD: u64 = 0x4E53_4512;
        static NS_APP_DID_FIN: u64 = 0x4E53_4513;
        // NSKeyValueChange keys
        static NS_KVC_NEW: u64 = 0x4E53_4514;
        static NS_KVC_OLD: u64 = 0x4E53_4515;
        static NS_KVC_KIND: u64 = 0x4E53_4516;
        static NS_KVC_IDX: u64 = 0x4E53_4517;
        // NSXMLParser error domain
        static NS_XML_ERRD: u64 = 0x4E53_4518;
        // DVT private
        static DVT_ENV_BOOL: u64 = 0x4456_5401;

        self.symbols.insert(
            "_NSLocalizedDescriptionKey",
            &NS_LOC_DESC as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSLocalizedFailureReasonErrorKey",
            &NS_LOC_REASON as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSLocalizedRecoveryOptionsErrorKey",
            &NS_LOC_REC as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSLocalizedRecoveryOptionsErrorKey",
            &NS_LOC_REC as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSLocalizedFailureSuggestionErrorKey",
            &NS_LOC_FAIL as *const u64 as u64,
        );
        self.symbols
            .insert("_NSUnderlyingErrorKey", &NS_UNDERLYING as *const u64 as u64);
        self.symbols
            .insert("_NSURLErrorKey", &NS_URL_ERR as *const u64 as u64);
        self.symbols
            .insert("_NSFilePathErrorKey", &NS_FILE_PATH as *const u64 as u64);
        self.symbols.insert(
            "_NSStringEncodingErrorKey",
            &NS_STR_ERR as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSDebugDescriptionErrorKey",
            &NS_DEBUGDESC as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSProcessInfoThermalStateDidChangeNotification",
            &NS_PROC_INFO as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSBundleDidLoadNotification",
            &NS_BUNDLE_DID_LOAD as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSApplicationDidFinishLaunchingNotification",
            &NS_APP_DID_FIN as *const u64 as u64,
        );
        self.symbols
            .insert("_NSKeyValueChangeNewKey", &NS_KVC_NEW as *const u64 as u64);
        self.symbols
            .insert("_NSKeyValueChangeOldKey", &NS_KVC_OLD as *const u64 as u64);
        self.symbols.insert(
            "_NSKeyValueChangeKindKey",
            &NS_KVC_KIND as *const u64 as u64,
        );
        self.symbols.insert(
            "_NSKeyValueChangeIndexesKey",
            &NS_KVC_IDX as *const u64 as u64,
        );
        self.symbols
            .insert("_NSXMLParserErrorDomain", &NS_XML_ERRD as *const u64 as u64);
        self.symbols.insert(
            "_DVTEnvironmentSnapshotBool",
            &DVT_ENV_BOOL as *const u64 as u64,
        );
    }

    // ── NStatManager key constants ────────────────────────────────

    fn register_nstat_keys(&mut self) {
        // NStatManager is a private networking statistics framework.
        // These key constants are used as dictionary keys only:
        // opaque non-NULL addresses prevent NULL-dereference.
        static NSTAT_MODE: u64 = 0x4E53_5401;
        static NSTAT_SRC_DESC: u64 = 0x4E53_5402;
        static NSTAT_COUNTS: u64 = 0x4E53_5403;
        static NSTAT_IFNET: u64 = 0x4E53_5404;
        static NSTAT_ROUTE: u64 = 0x4E53_5405;
        static NSTAT_TCP: u64 = 0x4E53_5406;
        static NSTAT_UDP: u64 = 0x4E53_5407;
        static NSTAT_QUIC: u64 = 0x4E53_5408;
        static NSTAT_PROVIDER: u64 = 0x4E53_5409;
        static NSTAT_BW_EST: u64 = 0x4E53_540A;
        static NSTAT_CONN_TYPE: u64 = 0x4E53_540B;
        static NSTAT_FLAGS: u64 = 0x4E53_540C;

        self.symbols
            .insert("_kNStatManagerMode", &NSTAT_MODE as *const u64 as u64);
        self.symbols.insert(
            "_kNStatSourceDescription",
            &NSTAT_SRC_DESC as *const u64 as u64,
        );
        self.symbols
            .insert("_kNStatCounts", &NSTAT_COUNTS as *const u64 as u64);
        self.symbols
            .insert("_kNStatProviderIFNet", &NSTAT_IFNET as *const u64 as u64);
        self.symbols
            .insert("_kNStatProviderRoute", &NSTAT_ROUTE as *const u64 as u64);
        self.symbols
            .insert("_kNStatProviderTCPv4", &NSTAT_TCP as *const u64 as u64);
        self.symbols
            .insert("_kNStatProviderTCPv6", &NSTAT_TCP as *const u64 as u64);
        self.symbols
            .insert("_kNStatProviderUDPv4", &NSTAT_UDP as *const u64 as u64);
        self.symbols
            .insert("_kNStatProviderUDPv6", &NSTAT_UDP as *const u64 as u64);
        self.symbols
            .insert("_kNStatProviderQUIC", &NSTAT_QUIC as *const u64 as u64);
        self.symbols.insert(
            "_kNStatManagerProvider",
            &NSTAT_PROVIDER as *const u64 as u64,
        );
        self.symbols.insert(
            "_kNStatManagerBandwidthEstimate",
            &NSTAT_BW_EST as *const u64 as u64,
        );
        self.symbols.insert(
            "_kNStatConnectionType",
            &NSTAT_CONN_TYPE as *const u64 as u64,
        );
        self.symbols
            .insert("_kNStatFlags", &NSTAT_FLAGS as *const u64 as u64);
    }

    // ── Swift runtime global singletons ───────────────────────────

    fn register_swift_runtime_globals(&mut self) {
        // ObjC/Swift empty-collection singletons are live ObjC objects whose
        // ISA field (first 8 bytes) must be a valid class pointer.  Providing
        // a zero-filled fake causes an immediate NULL-dereference when the
        // runtime reads the ISA.
        //
        // Foundation is already loaded in the host process (via StubResolver
        // preloads), so dlsym(RTLD_DEFAULT, ...) resolves these to the real
        // system singletons — including their correct ISA pointers.
        //
        // dlsym convention on macOS: the symbol name passed to dlsym is the
        // C-level name, which is the Mach-O symbol name minus the one leading
        // underscore the assembler adds.  Thus:
        //   Mach-O ___NSArray0__struct → dlsym "__NSArray0__struct"
        //   Mach-O __swiftEmptyArrayStorage → dlsym "_swiftEmptyArrayStorage"
        self.try_dlsym_insert("___NSArray0__struct", "__NSArray0__struct");
        self.try_dlsym_insert("___NSDictionary0__struct", "__NSDictionary0__struct");
        self.try_dlsym_insert("__swiftEmptyArrayStorage", "_swiftEmptyArrayStorage");
        self.try_dlsym_insert(
            "__swiftEmptyDictionarySingleton",
            "_swiftEmptyDictionarySingleton",
        );
        self.try_dlsym_insert("__swiftImmortalRefCount", "_swiftImmortalRefCount");
        self.try_dlsym_insert("__swiftEmptySetSingleton", "_swiftEmptySetSingleton");

        // On Linux, no host Foundation/Swift runtime → provide static
        // fallback singletons.  These are opaque data blobs whose exact
        // layout doesn't matter as long as callers get a valid non-NULL
        // address to compare against.
        #[cfg(not(target_os = "macos"))]
        {
            // Swift heap-object layout: 8-byte metadata pointer + 8-byte refcount
            // We fake these with zeroed buffers — callers check `== emptyArray`
            // rather than reading contents.
            static EMPTY_ARRAY: [u8; 64] = [0u8; 64];
            static EMPTY_DICT: [u8; 64] = [0u8; 64];
            static SWIFT_EMPTY_ARRAY: [u8; 64] = [0u8; 64];
            static SWIFT_EMPTY_DICT: [u8; 64] = [0u8; 64];
            static SWIFT_IMMORTAL_RC: u64 = 0;
            static SWIFT_EMPTY_SET: [u8; 64] = [0u8; 64];

            self.symbols
                .entry("__NSArray0__struct")
                .or_insert(EMPTY_ARRAY.as_ptr() as u64);
            self.symbols
                .entry("__NSDictionary0__struct")
                .or_insert(EMPTY_DICT.as_ptr() as u64);
            self.symbols
                .entry("_swiftEmptyArrayStorage")
                .or_insert(SWIFT_EMPTY_ARRAY.as_ptr() as u64);
            self.symbols
                .entry("_swiftEmptyDictionarySingleton")
                .or_insert(SWIFT_EMPTY_DICT.as_ptr() as u64);
            self.symbols
                .entry("_swiftImmortalRefCount")
                .or_insert(&SWIFT_IMMORTAL_RC as *const u64 as u64);
            self.symbols
                .entry("_swiftEmptySetSingleton")
                .or_insert(SWIFT_EMPTY_SET.as_ptr() as u64);
        }
    }

    // ── ObjC runtime globals ─────────────────────────────────────

    fn register_objc_runtime_globals(&mut self) {
        // __objc_empty_cache — initial method-cache sentinel for all ObjC
        // classes. Must be a valid non-NULL struct; the runtime compares its
        // address rather than reading its contents for dispatch.
        self.try_dlsym_insert("__objc_empty_cache", "_objc_empty_cache");
        self.try_dlsym_insert("__objc_empty_vtable", "_objc_empty_vtable");

        // Tagged-pointer class tables — arrays of Class pointers maintained
        // by the ObjC runtime. dlsym resolves these from libobjc.
        self.try_dlsym_insert(
            "__objc_debug_taggedpointer_classes",
            "_objc_debug_taggedpointer_classes",
        );
        self.try_dlsym_insert(
            "__objc_debug_taggedpointer_ext_classes",
            "_objc_debug_taggedpointer_ext_classes",
        );

        // On Linux, no host ObjC runtime exists so dlsym returns NULL above.
        // Provide static fallback buffers for required data symbols.
        #[cfg(not(target_os = "macos"))]
        {
            // 128 bytes of zeros — enough for the cache/vtable sentinels.
            // The runtime only compares the address, not the contents.
            static EMPTY_CACHE: [u8; 128] = [0u8; 128];
            static EMPTY_VTABLE: [u8; 64] = [0u8; 64];
            // Tagged-pointer class tables: array of 8 Class pointers (all NULL)
            static TAGGED_CLASSES: [u64; 8] = [0u64; 8];
            static TAGGED_EXT_CLASSES: [u64; 256] = [0u64; 256];

            self.symbols
                .entry("_objc_empty_cache")
                .or_insert(EMPTY_CACHE.as_ptr() as u64);
            self.symbols
                .entry("_objc_empty_vtable")
                .or_insert(EMPTY_VTABLE.as_ptr() as u64);
            self.symbols
                .entry("_objc_debug_taggedpointer_classes")
                .or_insert(TAGGED_CLASSES.as_ptr() as u64);
            self.symbols
                .entry("_objc_debug_taggedpointer_ext_classes")
                .or_insert(TAGGED_EXT_CLASSES.as_ptr() as u64);
        }
    }

    // ── ObjC IVAR offset data symbols ───────────────────────────────

    fn register_objc_ivar_offsets(&mut self) {
        // Each _OBJC_IVAR_$_ClassName.fieldName symbol is a pointer to a
        // u64 containing the byte offset of that ivar within the class
        // instance.  Guest code loads from this address to compute field
        // pointers:  `ldr x0, [got_ivar]; ldr x1, [obj, x0]`
        //
        // The offsets here must match our cf-lite shim object layouts.

        // NSError
        static NSERROR_CODE: u64 = 8;
        static NSERROR_DOMAIN: u64 = 16;
        static NSERROR_USERINFO: u64 = 24;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSError._code",
            &NSERROR_CODE as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSError._domain",
            &NSERROR_DOMAIN as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSError._userInfo",
            &NSERROR_USERINFO as *const u64 as u64,
        );

        // NSException
        static NSEXCEPTION_NAME: u64 = 8;
        static NSEXCEPTION_REASON: u64 = 16;
        static NSEXCEPTION_USERINFO: u64 = 24;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSException._name",
            &NSEXCEPTION_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSException._reason",
            &NSEXCEPTION_REASON as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSException._userInfo",
            &NSEXCEPTION_USERINFO as *const u64 as u64,
        );

        // NSURL
        static NSURL_STRING: u64 = 8;
        static NSURL_IS_FILE_URL: u64 = 16;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSURL._urlString",
            &NSURL_STRING as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSURL._isFileURL",
            &NSURL_IS_FILE_URL as *const u64 as u64,
        );

        // NSDate
        static NSDATE_TI: u64 = 8;

        self.symbols.insert(
            "_OBJC_IVAR_$___NSDate._timeIntervalSinceReferenceDate",
            &NSDATE_TI as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSDate._timeIntervalSinceReferenceDate",
            &NSDATE_TI as *const u64 as u64,
        );

        // NSNumber
        static NSNUMBER_TYPE: u64 = 8;
        static NSNUMBER_VAL: u64 = 16;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSNumber._type",
            &NSNUMBER_TYPE as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSNumber._value",
            &NSNUMBER_VAL as *const u64 as u64,
        );

        // NSData
        static NSDATA_LENGTH: u64 = 8;
        static NSDATA_BYTES: u64 = 16;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSData._length",
            &NSDATA_LENGTH as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSData._bytes",
            &NSDATA_BYTES as *const u64 as u64,
        );

        // NSArray
        static NSARRAY_COUNT: u64 = 8;
        static NSARRAY_ELEMENTS: u64 = 16;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSArray._count",
            &NSARRAY_COUNT as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSArray._elements",
            &NSARRAY_ELEMENTS as *const u64 as u64,
        );

        // NSDictionary
        static NSDICT_COUNT: u64 = 8;
        static NSDICT_KEYS: u64 = 16;
        static NSDICT_VALS: u64 = 24;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSDictionary._count",
            &NSDICT_COUNT as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSDictionary._keys",
            &NSDICT_KEYS as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSDictionary._values",
            &NSDICT_VALS as *const u64 as u64,
        );

        // NSThread
        static NSTHREAD_NAME: u64 = 8;
        static NSTHREAD_TARGET: u64 = 16;
        static NSTHREAD_SEL: u64 = 24;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSThread._name",
            &NSTHREAD_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSThread._target",
            &NSTHREAD_TARGET as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSThread._selector",
            &NSTHREAD_SEL as *const u64 as u64,
        );

        // NSNotification
        static NSNOTIF_NAME: u64 = 8;
        static NSNOTIF_OBJECT: u64 = 16;
        static NSNOTIF_USERINFO: u64 = 24;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSNotification._name",
            &NSNOTIF_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSNotification._object",
            &NSNOTIF_OBJECT as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_NSNotification._userInfo",
            &NSNOTIF_USERINFO as *const u64 as u64,
        );

        // NSBundle
        static NSBUNDLE_PATH: u64 = 8;

        self.symbols.insert(
            "_OBJC_IVAR_$_NSBundle._bundlePath",
            &NSBUNDLE_PATH as *const u64 as u64,
        );
    }

    // ── DVTFoundation / IDEFoundation constant data symbols ─────────

    fn register_dvt_ide_constants(&mut self) {
        // DVT internal constants — opaque non-NULL sentinels.
        // These are used as dictionary keys, notification names, or
        // configuration flags by Xcode tool init paths.
        //
        // Sentinel range: 0x4456_54xx ("DVT" prefix)
        static DVT_CURRENT_ARCH: u64 = 0x4456_5402;
        static DVT_PLUGIN_COMPAT_UUID: u64 = 0x4456_5403;
        static DVT_PLUGIN_EXTENSION_KEY: u64 = 0x4456_5404;
        static DVT_DEFAULT_TOOLCHAIN: u64 = 0x4456_5405;
        static DVT_PLATFORM_FAMILY: u64 = 0x4456_5406;
        static DVT_BUILD_SETTING_KEY: u64 = 0x4456_5407;
        static DVT_FS_EVENT_LATENCY: u64 = 0x4456_5408;

        self.symbols.insert(
            "_kDVTCurrentArchitecture",
            &DVT_CURRENT_ARCH as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDVTPlugInCompatibilityUUID",
            &DVT_PLUGIN_COMPAT_UUID as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDVTPlugInExtensionPointKey",
            &DVT_PLUGIN_EXTENSION_KEY as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDVTDefaultToolchainIdentifier",
            &DVT_DEFAULT_TOOLCHAIN as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDVTPlatformFamilyKey",
            &DVT_PLATFORM_FAMILY as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDVTBuildSettingKey",
            &DVT_BUILD_SETTING_KEY as *const u64 as u64,
        );
        self.symbols.insert(
            "_kDVTFileSystemEventStreamLatencyKey",
            &DVT_FS_EVENT_LATENCY as *const u64 as u64,
        );

        // IDE internal constants — sentinel range: 0x4944_45xx ("IDE" prefix)
        static IDE_INDEX_DB_DIR: u64 = 0x4944_4501;
        static IDE_WORKSPACE_CHECK_KEY: u64 = 0x4944_4502;
        static IDE_SOURCE_TREE_ROOT: u64 = 0x4944_4503;
        static IDE_BUILD_OP_SUCCEEDED: u64 = 0x4944_4504;
        static IDE_BUILD_OP_FAILED: u64 = 0x4944_4505;
        static IDE_ACTIVITY_LOG_SECTION: u64 = 0x4944_4506;

        self.symbols.insert(
            "_kIDEIndexDatabaseDirectoryKey",
            &IDE_INDEX_DB_DIR as *const u64 as u64,
        );
        self.symbols.insert(
            "_kIDEWorkspaceChecksKey",
            &IDE_WORKSPACE_CHECK_KEY as *const u64 as u64,
        );
        self.symbols.insert(
            "_kIDESourceTreeRootKey",
            &IDE_SOURCE_TREE_ROOT as *const u64 as u64,
        );
        self.symbols.insert(
            "_kIDEBuildOperationDidSucceedNotification",
            &IDE_BUILD_OP_SUCCEEDED as *const u64 as u64,
        );
        self.symbols.insert(
            "_kIDEBuildOperationDidFailNotification",
            &IDE_BUILD_OP_FAILED as *const u64 as u64,
        );
        self.symbols.insert(
            "_kIDEActivityLogSectionKey",
            &IDE_ACTIVITY_LOG_SECTION as *const u64 as u64,
        );

        // DVT OBJC_IVAR offsets — DVT classes have ISA at 0, then fields.
        // DVTFilePath: [ISA:8][pathString:8][parent:8]
        static DVTFILEPATH_PATH: u64 = 8;
        static DVTFILEPATH_PARENT: u64 = 16;

        self.symbols.insert(
            "_OBJC_IVAR_$_DVTFilePath._pathString",
            &DVTFILEPATH_PATH as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_DVTFilePath._parentPath",
            &DVTFILEPATH_PARENT as *const u64 as u64,
        );

        // DVTLogAspect: [ISA:8][name:8][level:8]
        static DVTLOGASPECT_NAME: u64 = 8;
        static DVTLOGASPECT_LEVEL: u64 = 16;

        self.symbols.insert(
            "_OBJC_IVAR_$_DVTLogAspect._name",
            &DVTLOGASPECT_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_DVTLogAspect._level",
            &DVTLOGASPECT_LEVEL as *const u64 as u64,
        );

        // DVTPlatform: [ISA:8][name:8][identifier:8]
        static DVTPLATFORM_NAME: u64 = 8;
        static DVTPLATFORM_IDENT: u64 = 16;

        self.symbols.insert(
            "_OBJC_IVAR_$_DVTPlatform._name",
            &DVTPLATFORM_NAME as *const u64 as u64,
        );
        self.symbols.insert(
            "_OBJC_IVAR_$_DVTPlatform._identifier",
            &DVTPLATFORM_IDENT as *const u64 as u64,
        );
    }

    /// Resolve a symbol via `dlsym(RTLD_DEFAULT, c_name)` and insert it.
    /// No-ops silently if the symbol is not found (binary will fall through
    /// to the StubResolver which may also resolve it, or leave it as NULL).
    fn try_dlsym_insert(&mut self, mach_o_sym: &'static str, c_sym: &str) {
        let Ok(cstr) = std::ffi::CString::new(c_sym) else {
            return;
        };
        let addr = host_process_symbol(&cstr);
        if !addr.is_null() {
            log::debug!(
                "DataSymbolStore: {} → {:#x} (via dlsym)",
                mach_o_sym,
                addr as u64
            );
            self.symbols.insert(mach_o_sym, addr as u64);
        } else {
            log::debug!("DataSymbolStore: dlsym('{}') → not found", c_sym);
        }
    }
}

#[cfg(unix)]
fn host_process_symbol(cstr: &std::ffi::CStr) -> *mut libc::c_void {
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, cstr.as_ptr()) }
}

#[cfg(not(unix))]
fn host_process_symbol(_cstr: &std::ffi::CStr) -> *mut libc::c_void {
    std::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_symbol_store_resolves_xpc_types() {
        let store = DataSymbolStore::new();

        // All XPC type symbols should resolve to non-NULL addresses
        let xpc_types = [
            "__xpc_type_activity",
            "__xpc_type_array",
            "__xpc_type_bool",
            "__xpc_type_connection",
            "__xpc_type_data",
            "__xpc_type_dictionary",
            "__xpc_type_error",
            "__xpc_type_string",
        ];

        for name in &xpc_types {
            let addr = store.resolve(name);
            assert!(addr.is_some(), "expected {name} to resolve");
            assert_ne!(addr.unwrap(), 0, "expected {name} to be non-NULL");
        }

        // Each type should have a unique address
        let addrs: Vec<u64> = xpc_types
            .iter()
            .map(|n| store.resolve(n).unwrap())
            .collect();
        for i in 0..addrs.len() {
            for j in (i + 1)..addrs.len() {
                assert_ne!(
                    addrs[i], addrs[j],
                    "XPC types {} and {} must have unique addresses",
                    xpc_types[i], xpc_types[j]
                );
            }
        }
    }

    #[test]
    fn data_symbol_store_resolves_dispatch_source_types() {
        let store = DataSymbolStore::new();

        let names = [
            "__dispatch_source_type_data_add",
            "__dispatch_source_type_timer",
            "__dispatch_source_type_memorypressure",
        ];

        for name in &names {
            let addr = store.resolve(name);
            assert!(addr.is_some(), "expected {name} to resolve");
            assert_ne!(addr.unwrap(), 0, "expected {name} to be non-NULL");
        }
    }

    #[test]
    fn data_symbol_store_resolves_ndr_record() {
        let store = DataSymbolStore::new();
        let addr = store
            .resolve("_NDR_record")
            .expect("NDR_record should resolve");
        assert_ne!(addr, 0);

        // Verify the NDR record contents
        let ptr = addr as *const u8;
        unsafe {
            assert_eq!(*ptr, 0); // mig_vers
            assert_eq!(*ptr.add(3), 1); // mig_encoding
            assert_eq!(*ptr.add(4), 1); // int_rep (little-endian)
        }
    }

    #[test]
    fn data_symbol_store_resolves_sys_signame() {
        let store = DataSymbolStore::new();
        let addr = store
            .resolve("_sys_signame")
            .expect("sys_signame should resolve");
        assert_ne!(addr, 0);
    }

    #[test]
    fn data_symbol_store_resolves_cf_version_keys() {
        let store = DataSymbolStore::new();
        for name in &[
            "__kCFSystemVersionProductNameKey",
            "__kCFSystemVersionProductVersionKey",
            "__kCFSystemVersionBuildVersionKey",
        ] {
            assert!(store.resolve(name).is_some(), "{name} should resolve");
        }
    }

    #[test]
    fn data_symbol_store_returns_none_for_unknown() {
        let store = DataSymbolStore::new();
        assert!(store.resolve("_nonexistent_symbol").is_none());
    }

    #[test]
    fn data_symbol_store_has_expected_count() {
        let store = DataSymbolStore::new();
        // 16 XPC types + 5 XPC errors + 12 dispatch sources + 1 NDR + 1 sys_signame + 4 CF version
        // + foundation constants + block globals + DA keys + LS keys + NS constants + NStatManager
        assert!(
            store.len() >= 100,
            "expected at least 100 data symbols, got {}",
            store.len()
        );
    }

    #[test]
    fn foundation_constants_non_null() {
        let store = DataSymbolStore::new();
        let expected = [
            "___kCFBooleanTrue",
            "___kCFBooleanFalse",
            "___CFConstantStringClassReference",
            "_kCFRunLoopDefaultMode",
            "_kCFRunLoopCommonModes",
            "_NSDefaultRunLoopMode",
            "_NSRunLoopCommonModes",
        ];
        for sym in &expected {
            let addr = store.resolve(sym);
            assert!(addr.is_some(), "{sym} should resolve");
            assert_ne!(addr.unwrap(), 0, "{sym} must be non-NULL");
        }
    }

    #[test]
    fn block_globals_non_null() {
        let store = DataSymbolStore::new();
        let expected = ["___NSConcreteGlobalBlock", "___NSConcreteStackBlock"];
        for sym in &expected {
            let addr = store.resolve(sym);
            assert!(addr.is_some(), "{sym} should resolve");
            assert_ne!(addr.unwrap(), 0, "{sym} must be non-NULL");
        }
    }

    #[test]
    fn ns_error_keys_non_null() {
        let store = DataSymbolStore::new();
        let addr = store.resolve("_NSLocalizedDescriptionKey");
        assert!(addr.is_some(), "_NSLocalizedDescriptionKey should resolve");
        assert_ne!(addr.unwrap(), 0);
    }

    #[test]
    fn da_keys_non_null() {
        let store = DataSymbolStore::new();
        let addr = store.resolve("_kDADiskDescriptionMediaNameKey");
        assert!(
            addr.is_some(),
            "_kDADiskDescriptionMediaNameKey should resolve"
        );
        assert_ne!(addr.unwrap(), 0);
    }

    #[test]
    fn objc_ivar_offsets_resolve() {
        let store = DataSymbolStore::new();
        // Each IVAR symbol resolves to a pointer whose target is the byte offset
        let cases = [
            ("_OBJC_IVAR_$_NSError._code", 8u64),
            ("_OBJC_IVAR_$_NSError._domain", 16),
            ("_OBJC_IVAR_$_NSError._userInfo", 24),
            ("_OBJC_IVAR_$_NSData._length", 8),
            ("_OBJC_IVAR_$_NSData._bytes", 16),
            ("_OBJC_IVAR_$_NSArray._count", 8),
            ("_OBJC_IVAR_$_NSDictionary._count", 8),
            ("_OBJC_IVAR_$_NSURL._urlString", 8),
            ("_OBJC_IVAR_$_NSDate._timeIntervalSinceReferenceDate", 8),
            ("_OBJC_IVAR_$_NSBundle._bundlePath", 8),
        ];
        for (sym, expected_offset) in &cases {
            let addr = store.resolve(sym);
            assert!(addr.is_some(), "{sym} should resolve");
            let ptr = addr.unwrap() as *const u64;
            let offset = unsafe { *ptr };
            assert_eq!(
                offset, *expected_offset,
                "{sym}: expected offset {expected_offset}, got {offset}"
            );
        }
    }
}
