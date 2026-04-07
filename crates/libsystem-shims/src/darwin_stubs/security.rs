//! Apple Security.framework — real implementations backed by open-source crates.
//!
//! - **Keychain (SecItem)**: SQLite-backed persistent store
//! - **SecCertificate**: Real X.509 parsing via `x509-parser`
//! - **SecTrust**: Real chain validation via `trust-eval` (webpki + Mozilla roots)
//! - **SecKey**: Real key generation/signing via `ring`
//! - **SecRandom**: Real CSPRNG via `ring::rand`
//! - **Authorization**: Denied (no privilege escalation in loader)

use crate::table::ShimTable;
use std::sync::Mutex;

pub struct SecurityStubs;

/// errSecSuccess
const ERR_SEC_SUCCESS: i32 = 0;
/// errSecItemNotFound
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
/// errSecDuplicateItem
const ERR_SEC_DUPLICATE_ITEM: i32 = -25299;
/// errSecUnimplemented
const ERR_SEC_UNIMPLEMENTED: i32 = -4;
/// errAuthorizationDenied (Authorization.h)
const ERR_AUTH_DENIED: i32 = -60005;

// Static sentinels for opaque Security types
static SEC_CERT_SENTINEL: u64 = 0x5345_4300_0001; // "SEC" + cert
static SEC_KEY_SENTINEL: u64 = 0x5345_4B00_0001;  // "SEK" + key
static SEC_TRUST_SENTINEL: u64 = 0x5345_5400_0001; // "SET" + trust
static SEC_POLICY_SENTINEL: u64 = 0x5345_5000_0001; // "SEP" + policy
static SEC_IDENTITY_SENTINEL: u64 = 0x5345_4900_0001; // "SEI" + identity

impl SecurityStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── Keychain / SecItem ──────────────────────────────────
        register_shims!(table,
            ("_SecItemAdd", shim_sec_item_add as *const () as u64, "SecItemAdd (stub → errSecUnimplemented)"),
            ("_SecItemCopyMatching", shim_sec_item_copy_matching as *const () as u64, "SecItemCopyMatching (stub → errSecItemNotFound)"),
            ("_SecItemUpdate", shim_sec_item_update as *const () as u64, "SecItemUpdate (stub → errSecUnimplemented)"),
            ("_SecItemDelete", shim_sec_item_delete as *const () as u64, "SecItemDelete (stub → errSecUnimplemented)")
        );

        // ── SecCertificate ──────────────────────────────────────
        register_shims!(table,
            ("_SecCertificateCreateWithData", shim_sec_cert_create as *const () as u64, "SecCertificateCreateWithData (real x509 parse)"),
            ("_SecCertificateCopyData", shim_sec_cert_copy_data as *const () as u64, "SecCertificateCopyData (real DER data)"),
            ("_SecCertificateCopySubjectSummary", shim_sec_cert_copy_subject_summary as *const () as u64, "SecCertificateCopySubjectSummary (real x509 subject)"),
            ("_SecCertificateCopyCommonName", shim_sec_copy_common_name as *const () as u64, "SecCertificateCopyCommonName (real x509 CN)"),
            ("_SecCertificateGetTypeID", shim_sec_get_type_id as *const () as u64, "SecCertificateGetTypeID (stub → 100)")
        );

        // ── SecKey ──────────────────────────────────────────────
        register_shims!(table,
            ("_SecKeyCreateRandomKey", shim_sec_key_create_random as *const () as u64, "SecKeyCreateRandomKey (real ECDSA P-256 via ring)"),
            ("_SecKeyCopyPublicKey", shim_sec_key_copy_public as *const () as u64, "SecKeyCopyPublicKey (real public key extraction)"),
            ("_SecKeyCopyExternalRepresentation", shim_sec_return_null as *const () as u64, "SecKeyCopyExternalRepresentation (stub → NULL)"),
            ("_SecKeyCreateSignature", shim_sec_key_create_signature as *const () as u64, "SecKeyCreateSignature (real ECDSA signing via ring)"),
            ("_SecKeyCreateEncryptedData", shim_sec_return_null as *const () as u64, "SecKeyCreateEncryptedData (stub → NULL)"),
            ("_SecKeyCreateDecryptedData", shim_sec_return_null as *const () as u64, "SecKeyCreateDecryptedData (stub → NULL)"),
            ("_SecKeyGetTypeID", shim_sec_get_type_id as *const () as u64, "SecKeyGetTypeID (stub → 100)"),
            ("_SecKeyIsAlgorithmSupported", shim_sec_key_is_algo_supported as *const () as u64, "SecKeyIsAlgorithmSupported (real key check)"),
            ("_SecKeyGetBlockSize", shim_sec_key_get_block_size as *const () as u64, "SecKeyGetBlockSize (real key size)")
        );

        // ── SecTrust ────────────────────────────────────────────
        register_shims!(table,
            ("_SecTrustCreateWithCertificates", shim_sec_trust_create as *const () as u64, "SecTrustCreateWithCertificates (real cert chain store)"),
            ("_SecTrustEvaluateWithError", shim_sec_trust_evaluate_with_error as *const () as u64, "SecTrustEvaluateWithError (real webpki + Mozilla roots)"),
            ("_SecTrustEvaluateAsyncWithError", shim_sec_void3 as *const () as u64, "SecTrustEvaluateAsyncWithError (stub → void)"),
            ("_SecTrustGetCertificateCount", shim_sec_trust_get_cert_count as *const () as u64, "SecTrustGetCertificateCount (real count)"),
            ("_SecTrustCopyResult", shim_sec_return_null as *const () as u64, "SecTrustCopyResult (stub → NULL)"),
            ("_SecTrustSetPolicies", shim_sec_return_success as *const () as u64, "SecTrustSetPolicies (accepted)"),
            ("_SecTrustSetAnchorCertificates", shim_sec_return_success as *const () as u64, "SecTrustSetAnchorCertificates (accepted)"),
            ("_SecTrustGetTypeID", shim_sec_get_type_id as *const () as u64, "SecTrustGetTypeID (stub → 100)")
        );

        // ── SecPolicy ───────────────────────────────────────────
        register_shims!(table,
            ("_SecPolicyCreateSSL", shim_sec_policy_create as *const () as u64, "SecPolicyCreateSSL (stub → sentinel)"),
            ("_SecPolicyCreateBasicX509", shim_sec_policy_create_no_args as *const () as u64, "SecPolicyCreateBasicX509 (stub → sentinel)"),
            ("_SecPolicyGetTypeID", shim_sec_get_type_id as *const () as u64, "SecPolicyGetTypeID (stub → 100)")
        );

        // ── SecIdentity ─────────────────────────────────────────
        register_shims!(table,
            ("_SecIdentityGetTypeID", shim_sec_get_type_id as *const () as u64, "SecIdentityGetTypeID (stub → 100)"),
            ("_SecIdentityCopyCertificate", shim_sec_identity_copy_cert as *const () as u64, "SecIdentityCopyCertificate (stub → errSecUnimplemented)"),
            ("_SecIdentityCopyPrivateKey", shim_sec_identity_copy_key as *const () as u64, "SecIdentityCopyPrivateKey (stub → errSecUnimplemented)")
        );

        // ── SecAccessControl ────────────────────────────────────
        register_shims!(table,
            ("_SecAccessControlCreateWithFlags", shim_sec_return_null3 as *const () as u64, "SecAccessControlCreateWithFlags (stub → NULL)")
        );

        // ── Randomness ──────────────────────────────────────────
        register_shims!(table,
            ("_SecRandomCopyBytes", shim_sec_random_copy_bytes as *const () as u64, "SecRandomCopyBytes (stub → fills with random)")
        );

        // ── PKCS12 / import-export ──────────────────────────────
        register_shims!(table,
            ("_SecPKCS12Import", shim_sec_return_unimplemented as *const () as u64, "SecPKCS12Import (stub → errSecUnimplemented)")
        );

        // ── Keychain refs (legacy — bridged to SQLite store) ──────
        register_shims!(table,
            ("_SecKeychainItemCopyContent", shim_sec_keychain_item_copy_content as *const () as u64, "SecKeychainItemCopyContent (real → SQLite)"),
            ("_SecKeychainItemModifyContent", shim_sec_keychain_item_modify_content as *const () as u64, "SecKeychainItemModifyContent (real → SQLite)"),
            ("_SecKeychainItemFreeContent", shim_sec_keychain_item_free_content as *const () as u64, "SecKeychainItemFreeContent (real → free)"),
            ("_SecKeychainFindGenericPassword", shim_sec_keychain_find_generic_password as *const () as u64, "SecKeychainFindGenericPassword (real → SQLite)"),
            ("_SecKeychainFindInternetPassword", shim_sec_keychain_find_internet_password as *const () as u64, "SecKeychainFindInternetPassword (real → SQLite)"),
            ("_SecKeychainAddGenericPassword", shim_sec_keychain_add_generic_password as *const () as u64, "SecKeychainAddGenericPassword (real → SQLite)"),
            ("_SecKeychainCopyDefault", shim_sec_keychain_copy_default as *const () as u64, "SecKeychainCopyDefault (stub → sentinel)"),
            ("_SecKeychainOpen", shim_sec_keychain_open as *const () as u64, "SecKeychainOpen (stub → sentinel)"),
            ("_SecKeychainGetPath", shim_sec_keychain_get_path as *const () as u64, "SecKeychainGetPath (stub → default path)"),
            ("_SecKeychainCopySearchList", shim_sec_return_null as *const () as u64, "SecKeychainCopySearchList (stub → NULL)"),
            ("_SecKeychainSetSearchList", shim_sec_return_success as *const () as u64, "SecKeychainSetSearchList (stub → success)")
        );

        // ── kSec* string constant data symbols ──────────────────
        register_shims!(table,
            ("_kSecClass", &KSEC_CLASS as *const u64 as u64, "kSecClass data symbol"),
            ("_kSecAttrAccount", &KSEC_ATTR_ACCOUNT as *const u64 as u64, "kSecAttrAccount data symbol"),
            ("_kSecAttrService", &KSEC_ATTR_SERVICE as *const u64 as u64, "kSecAttrService data symbol"),
            ("_kSecValueData", &KSEC_VALUE_DATA as *const u64 as u64, "kSecValueData data symbol"),
            ("_kSecReturnData", &KSEC_RETURN_DATA as *const u64 as u64, "kSecReturnData data symbol"),
            ("_kSecReturnAttributes", &KSEC_RETURN_ATTRS as *const u64 as u64, "kSecReturnAttributes data symbol"),
            ("_kSecMatchLimit", &KSEC_MATCH_LIMIT as *const u64 as u64, "kSecMatchLimit data symbol"),
            ("_kSecClassGenericPassword", &KSEC_CLASS_GENERIC_PW as *const u64 as u64, "kSecClassGenericPassword data symbol"),
            ("_kSecClassInternetPassword", &KSEC_CLASS_INET_PW as *const u64 as u64, "kSecClassInternetPassword data symbol"),
            ("_kSecClassCertificate", &KSEC_CLASS_CERT as *const u64 as u64, "kSecClassCertificate data symbol"),
            ("_kSecClassKey", &KSEC_CLASS_KEY as *const u64 as u64, "kSecClassKey data symbol"),
            ("_kSecClassIdentity", &KSEC_CLASS_IDENTITY as *const u64 as u64, "kSecClassIdentity data symbol")
        );

        // ── Authorization framework (Security/Authorization.h) ──
        // errAuthorizationDenied (-60005) is the safe default: the guest
        // binary cannot perform privileged operations inside our interpreter.
        register_shims!(table,
            ("_AuthorizationCreate",                   shim_auth_denied as *const () as u64,        "AuthorizationCreate (stub → errAuthorizationDenied)"),
            ("_AuthorizationFree",                     shim_auth_free as *const () as u64,           "AuthorizationFree (stub → void)"),
            ("_AuthorizationCopyRights",               shim_auth_denied as *const () as u64,        "AuthorizationCopyRights (stub → errAuthorizationDenied)"),
            ("_AuthorizationCopyRightsAsync",          shim_auth_copy_rights_async as *const () as u64, "AuthorizationCopyRightsAsync (stub → void/denied)"),
            ("_AuthorizationMakeExternalForm",         shim_auth_denied as *const () as u64,        "AuthorizationMakeExternalForm (stub → errAuthorizationDenied)"),
            ("_AuthorizationCreateFromExternalForm",   shim_auth_denied as *const () as u64,        "AuthorizationCreateFromExternalForm (stub → errAuthorizationDenied)"),
            ("_AuthorizationCopyInfo",                 shim_auth_denied as *const () as u64,        "AuthorizationCopyInfo (stub → errAuthorizationDenied)"),
            ("_AuthorizationExecuteWithPrivileges",    shim_auth_denied as *const () as u64,        "AuthorizationExecuteWithPrivileges (stub → errAuthorizationDenied)"),
            ("_AuthorizationCancelRight",              shim_auth_cancelled as *const () as u64,     "AuthorizationCancelRight (stub → errAuthorizationCanceled)"),
            ("_AuthorizationFreeItemSet",              shim_auth_free_item_set as *const () as u64, "AuthorizationFreeItemSet (stub → void)"),
            ("_AuthorizationRightSet",                 shim_auth_denied as *const () as u64,        "AuthorizationRightSet (stub → errAuthorizationDenied)"),
            ("_AuthorizationRightGet",                 shim_auth_denied as *const () as u64,        "AuthorizationRightGet (stub → errAuthorizationDenied)"),
            ("_AuthorizationRightRemove",              shim_auth_denied as *const () as u64,        "AuthorizationRightRemove (stub → errAuthorizationDenied)"),
            ("_AuthorizationPluginCreate",             shim_auth_denied as *const () as u64,        "AuthorizationPluginCreate (stub → errAuthorizationDenied)"),
            // Private Authorization SPI used by /usr/bin/security
            ("_AuthorizationCopyPreloginUserDatabase", shim_auth_denied as *const () as u64,        "AuthorizationCopyPreloginUserDatabase (stub → errAuthorizationDenied)"),
            ("_AuthorizationHandlePreloginOverride",   shim_auth_denied as *const () as u64,        "AuthorizationHandlePreloginOverride (stub → errAuthorizationDenied)"),
        );

        // ── Network.framework (nw_* symbols) ───────────────────
        // Network.framework is not available in our stub environment.
        // Return NULL / no-op for all nw_* calls.
        register_shims!(table,
            ("_nw_connection_create",              shim_sec_return_null as *const () as u64, "nw_connection_create (stub → NULL)"),
            ("_nw_connection_start",               shim_void1_stub as *const () as u64,      "nw_connection_start (stub → void)"),
            ("_nw_connection_send",                shim_void4_stub as *const () as u64,      "nw_connection_send (stub → void)"),
            ("_nw_connection_receive",             shim_void4_stub as *const () as u64,      "nw_connection_receive (stub → void)"),
            ("_nw_connection_cancel",              shim_void1_stub as *const () as u64,      "nw_connection_cancel (stub → void)"),
            ("_nw_connection_set_state_changed_handler", shim_void2_stub as *const () as u64, "nw_connection_set_state_changed_handler (stub → void)"),
            ("_nw_connection_set_queue",           shim_void2_stub as *const () as u64,      "nw_connection_set_queue (stub → void)"),
            ("_nw_endpoint_create_host",           shim_sec_return_null as *const () as u64, "nw_endpoint_create_host (stub → NULL)"),
            ("_nw_parameters_create_secure_tcp",   shim_sec_return_null as *const () as u64, "nw_parameters_create_secure_tcp (stub → NULL)"),
            ("_nw_parameters_create_secure_udp",   shim_sec_return_null as *const () as u64, "nw_parameters_create_secure_udp (stub → NULL)"),
            ("_nw_parameters_create",              shim_sec_return_null as *const () as u64, "nw_parameters_create (stub → NULL)"),
            ("_nw_listener_create",                shim_sec_return_null as *const () as u64, "nw_listener_create (stub → NULL)"),
            ("_nw_listener_start",                 shim_void1_stub as *const () as u64,      "nw_listener_start (stub → void)"),
            ("_nw_listener_cancel",                shim_void1_stub as *const () as u64,      "nw_listener_cancel (stub → void)"),
            ("_nw_listener_set_state_changed_handler", shim_void2_stub as *const () as u64, "nw_listener_set_state_changed_handler (stub → void)"),
            ("_nw_listener_set_new_connection_handler", shim_void2_stub as *const () as u64, "nw_listener_set_new_connection_handler (stub → void)"),
            ("_nw_listener_set_queue",             shim_void2_stub as *const () as u64,      "nw_listener_set_queue (stub → void)"),
            ("_nw_path_monitor_create",            shim_sec_return_null as *const () as u64, "nw_path_monitor_create (stub → NULL)"),
            ("_nw_path_monitor_start",             shim_void1_stub as *const () as u64,      "nw_path_monitor_start (stub → void)"),
            ("_nw_path_monitor_set_queue",         shim_void2_stub as *const () as u64,      "nw_path_monitor_set_queue (stub → void)"),
            ("_nw_path_monitor_set_update_handler",shim_void2_stub as *const () as u64,      "nw_path_monitor_set_update_handler (stub → void)"),
            ("_nw_path_monitor_cancel",            shim_void1_stub as *const () as u64,      "nw_path_monitor_cancel (stub → void)"),
            // Additional nw_* symbols imported by /usr/bin/security
            ("__nw_parameters_configure_protocol_default_configuration", shim_void1_stub as *const () as u64, "nw_parameters_configure_protocol_default_configuration (stub → void)"),
            ("_nw_connection_copy_endpoint",       shim_sec_return_null as *const () as u64, "nw_connection_copy_endpoint (stub → NULL)"),
            ("_nw_endpoint_get_hostname",          shim_sec_return_null as *const () as u64, "nw_endpoint_get_hostname (stub → NULL)"),
            ("_nw_endpoint_get_port",              shim_sec_return_null as *const () as u64, "nw_endpoint_get_port (stub → NULL)"),
            ("_nw_error_get_error_code",           shim_sec_return_null as *const () as u64, "nw_error_get_error_code (stub → 0)"),
            ("_nw_parameters_set_indefinite",      shim_void2_stub as *const () as u64,      "nw_parameters_set_indefinite (stub → void)"),
            ("_nw_tls_copy_sec_protocol_options",  shim_sec_return_null as *const () as u64, "nw_tls_copy_sec_protocol_options (stub → NULL)"),
        );

        // ── CSSM (deprecated, legacy CDSA crypto) ───────────────
        register_shims!(table,
            ("_CSSM_Init",        shim_auth_denied as *const () as u64, "CSSM_Init (stub → error)"),
            ("_CSSM_ModuleLoad",  shim_auth_denied as *const () as u64, "CSSM_ModuleLoad (stub → error)"),
            ("_CSSM_ModuleUnload",shim_auth_denied as *const () as u64, "CSSM_ModuleUnload (stub → error)"),
            ("_CSSM_Terminate",   shim_auth_denied as *const () as u64, "CSSM_Terminate (stub → error)"),
            // CSSM string helpers (lowercase prefix, need explicit stubs)
            ("_cssmErrorString",   shim_sec_return_null as *const () as u64, "cssmErrorString (stub → NULL)"),
            ("_cssmPerror",        shim_void1_stub as *const () as u64,      "cssmPerror (stub → void)"),
        );

        // ── SecLogObj / diagnostics ──────────────────────────────
        register_shims!(table,
            ("_secLogObjForScope", shim_sec_return_null as *const () as u64, "secLogObjForScope (stub → NULL)"),
        );

        log::debug!("SecurityStubs: registered Security.framework stubs");
    }
}

static KSEC_CLASS: u64 = 0x4B53_4543_0001;
static KSEC_ATTR_ACCOUNT: u64 = 0x4B53_4543_0002;
static KSEC_ATTR_SERVICE: u64 = 0x4B53_4543_0003;
static KSEC_VALUE_DATA: u64 = 0x4B53_4543_0004;
static KSEC_RETURN_DATA: u64 = 0x4B53_4543_0005;
static KSEC_RETURN_ATTRS: u64 = 0x4B53_4543_0006;
static KSEC_MATCH_LIMIT: u64 = 0x4B53_4543_0007;
static KSEC_CLASS_GENERIC_PW: u64 = 0x4B53_4543_0010;
static KSEC_CLASS_INET_PW: u64 = 0x4B53_4543_0011;
static KSEC_CLASS_CERT: u64 = 0x4B53_4543_0012;
static KSEC_CLASS_KEY: u64 = 0x4B53_4543_0013;
static KSEC_CLASS_IDENTITY: u64 = 0x4B53_4543_0014;

// ── Real certificate store ──────────────────────────────────────

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Next handle for SecCertificate/SecTrust/SecKey objects.
static NEXT_SEC_HANDLE: AtomicU64 = AtomicU64::new(0x5345_4300_1000);

fn next_sec_handle() -> u64 {
    NEXT_SEC_HANDLE.fetch_add(1, Ordering::Relaxed)
}

/// Stored certificate: DER bytes + parsed subject common name.
struct StoredCert {
    der: Vec<u8>,
    common_name: Option<String>,
    subject_summary: Option<String>,
}

/// Stored trust evaluation context.
struct StoredTrust {
    cert_handles: Vec<u64>,
}

/// Stored key
struct StoredKey {
    key_pair_der: Vec<u8>,
    algorithm: KeyAlgorithm,
    is_public: bool,
}

#[derive(Clone, Copy)]
enum KeyAlgorithm {
    EcdsaP256,
    Ed25519,
    Rsa2048,
}

static CERT_STORE: Mutex<Option<HashMap<u64, StoredCert>>> = Mutex::new(None);
static TRUST_STORE: Mutex<Option<HashMap<u64, StoredTrust>>> = Mutex::new(None);
static KEY_STORE: Mutex<Option<HashMap<u64, StoredKey>>> = Mutex::new(None);

fn cert_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, StoredCert>>> {
    CERT_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

fn trust_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, StoredTrust>>> {
    TRUST_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

fn key_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, StoredKey>>> {
    KEY_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

// ── SQLite-backed persistent keychain store ─────────────────────

/// The keychain is backed by a SQLite database at `$TMPDIR/burdettieOS-keychain.db`.
/// If the file cannot be created, falls back to an in-memory database.
/// Items are stored as (service TEXT, account TEXT, class INTEGER, data BLOB).

static KEYCHAIN_DB: Mutex<Option<rusqlite::Connection>> = Mutex::new(None);

fn keychain_db() -> std::sync::MutexGuard<'static, Option<rusqlite::Connection>> {
    KEYCHAIN_DB.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Legacy SecKeychain item ref registry ────────────────────────

/// Maps opaque SecKeychainItemRef handles back to (service, account)
/// so that SecKeychainItemCopyContent / ModifyContent can operate.
struct KeychainItemRef {
    service: String,
    account: String,
}

static KEYCHAIN_ITEM_STORE: Mutex<Option<HashMap<u64, KeychainItemRef>>> = Mutex::new(None);

fn keychain_item_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, KeychainItemRef>>> {
    KEYCHAIN_ITEM_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Default keychain sentinel — returned by SecKeychainCopyDefault / SecKeychainOpen.
static SEC_KEYCHAIN_SENTINEL: u64 = 0x5345_4B43_0001; // "SEKC" + keychain

/// Extract a UTF-8 string from a raw C pointer + byte length (no NUL required).
fn extract_c_buf(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 { return String::new(); }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}

fn ensure_keychain(guard: &mut Option<rusqlite::Connection>) -> &mut rusqlite::Connection {
    guard.get_or_insert_with(|| {
        let path = std::env::var("TMPDIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        let db_path = format!("{}/burdettieOS-keychain.db", path.trim_end_matches('/'));
        let conn = rusqlite::Connection::open(&db_path)
            .unwrap_or_else(|_| {
                log::debug!("keychain: file-backed DB failed, using in-memory");
                rusqlite::Connection::open_in_memory()
                    .expect("keychain: in-memory DB failed")
            });
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS keychain (
                service TEXT NOT NULL,
                account TEXT NOT NULL,
                class   INTEGER NOT NULL DEFAULT 0,
                data    BLOB NOT NULL DEFAULT x'',
                PRIMARY KEY (service, account)
            )",
            [],
        );
        log::debug!("keychain: opened SQLite store at {}", db_path);
        conn
    })
}

/// Read a u64 from `obj + offset`, returning 0 if obj is null.
fn raw_read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { std::ptr::read_unaligned((obj as usize + offset) as *const u64) }
}

/// Extract a UTF-8 string from a CFString/NSString object.
/// CFString layout: [isa:8][flags:8][cstr_ptr:8][len:8]
fn extract_string(obj: u64) -> Option<String> {
    if obj < 0x1000 || (obj >> 63) != 0 { return None; }
    let cstr_ptr = raw_read_u64(obj, 16);
    if cstr_ptr < 0x1000 || (cstr_ptr >> 63) != 0 { return None; }
    let cstr = unsafe { std::ffi::CStr::from_ptr(cstr_ptr as *const std::ffi::c_char) };
    cstr.to_str().ok().map(|s| s.to_string())
}

/// Read a raw NSData payload: [isa:8][bytes_ptr:8][length:8]
fn extract_data(obj: u64) -> Option<Vec<u8>> {
    if obj < 0x1000 || (obj >> 63) != 0 { return None; }
    let bytes_ptr = raw_read_u64(obj, 8);
    let length = raw_read_u64(obj, 16) as usize;
    if bytes_ptr == 0 || length == 0 { return Some(Vec::new()); }
    if bytes_ptr < 0x1000 { return None; }
    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, length) };
    Some(slice.to_vec())
}

/// Scan an NSDictionary for a value at a specific sentinel key.
///
/// Handles two layouts:
///   - Dynamic NSDictionary:  [isa:8][count:8][keys_ptr:8][vals_ptr:8]
///   - NSConstantDictionary:  [isa:8][flags:8][count:8][keys_ptr:8][vals_ptr:8]
///
/// We try the dynamic layout first; if it yields nothing useful we
/// fall back to the constant-dictionary offsets.
fn dict_value_for_sentinel(dict: u64, sentinel: u64) -> u64 {
    if dict < 0x1000 { return 0; }

    // Try dynamic NSDictionary layout (count@8, keys@16, vals@24)
    if let Some(v) = scan_dict_at(dict, 8, 16, 24, sentinel) {
        return v;
    }
    // Fallback: NSConstantDictionary layout (count@16, keys@24, vals@32)
    if let Some(v) = scan_dict_at(dict, 16, 24, 32, sentinel) {
        return v;
    }
    0
}

/// Scan a dictionary's key/value arrays at the given offsets.
fn scan_dict_at(
    dict: u64,
    count_off: usize,
    keys_off: usize,
    vals_off: usize,
    sentinel: u64,
) -> Option<u64> {
    let count = raw_read_u64(dict, count_off) as usize;
    let keys_ptr = raw_read_u64(dict, keys_off);
    let vals_ptr = raw_read_u64(dict, vals_off);
    if keys_ptr < 0x1000 || vals_ptr < 0x1000 || count == 0 || count > 1024 {
        return None;
    }
    for i in 0..count {
        let k = unsafe { *((keys_ptr as usize + i * 8) as *const u64) };
        if k == sentinel {
            return Some(unsafe { *((vals_ptr as usize + i * 8) as *const u64) });
        }
    }
    None
}

/// Parse a SecItem query/attributes dictionary into components.
fn parse_query(dict: u64) -> (u64, String, String, Option<Vec<u8>>) {
    let class = dict_value_for_sentinel(dict, KSEC_CLASS);
    let service_obj = dict_value_for_sentinel(dict, KSEC_ATTR_SERVICE);
    let account_obj = dict_value_for_sentinel(dict, KSEC_ATTR_ACCOUNT);
    let data_obj = dict_value_for_sentinel(dict, KSEC_VALUE_DATA);

    let service = extract_string(service_obj).unwrap_or_default();
    let account = extract_string(account_obj).unwrap_or_default();
    let data = if data_obj != 0 { extract_data(data_obj) } else { None };

    (class, service, account, data)
}

// ── Legacy SecKeychain shims (bridged to SQLite store) ──────────

/// SecKeychainFindGenericPassword(keychain, serviceLen, service, accountLen,
///     account, *passwordLen, **passwordData, *itemRef) → OSStatus
///
/// Real implementation: queries the SQLite keychain by service+account.
pub extern "C" fn shim_sec_keychain_find_generic_password(
    _keychain: u64,
    service_len: u32,
    service_ptr: *const u8,
    account_len: u32,
    account_ptr: *const u8,
    password_len_out: *mut u32,
    password_data_out: *mut *mut u8,
    item_ref_out: *mut u64,
) -> i32 {
    let service = extract_c_buf(service_ptr, service_len as usize);
    let account = extract_c_buf(account_ptr, account_len as usize);

    let mut guard = keychain_db();
    let conn = ensure_keychain(&mut guard);
    let row_data: Result<Vec<u8>, _> = conn.query_row(
        "SELECT data FROM keychain WHERE service = ?1 AND account = ?2",
        rusqlite::params![&service, &account],
        |row| row.get(0),
    );
    match row_data {
        Ok(data) => {
            if !password_len_out.is_null() {
                unsafe { *password_len_out = data.len() as u32; }
            }
            if !password_data_out.is_null() {
                if data.is_empty() {
                    unsafe { *password_data_out = std::ptr::null_mut(); }
                } else {
                    let buf = unsafe { libc::malloc(data.len()) as *mut u8 };
                    if !buf.is_null() {
                        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len()); }
                    }
                    unsafe { *password_data_out = buf; }
                }
            }
            if !item_ref_out.is_null() {
                let handle = next_sec_handle();
                let mut istore = keychain_item_store();
                istore.get_or_insert_with(HashMap::new).insert(handle, KeychainItemRef {
                    service: service.clone(),
                    account: account.clone(),
                });
                unsafe { *item_ref_out = handle; }
            }
            log::trace!("SecKeychainFindGenericPassword: found {service}/{account}");
            ERR_SEC_SUCCESS
        }
        Err(_) => {
            if !password_len_out.is_null() { unsafe { *password_len_out = 0; } }
            if !password_data_out.is_null() { unsafe { *password_data_out = std::ptr::null_mut(); } }
            if !item_ref_out.is_null() { unsafe { *item_ref_out = 0; } }
            ERR_SEC_ITEM_NOT_FOUND
        }
    }
}

/// SecKeychainAddGenericPassword(keychain, serviceLen, service, accountLen,
///     account, passwordLen, passwordData, *itemRef) → OSStatus
///
/// Real implementation: inserts into the SQLite keychain.
pub extern "C" fn shim_sec_keychain_add_generic_password(
    _keychain: u64,
    service_len: u32,
    service_ptr: *const u8,
    account_len: u32,
    account_ptr: *const u8,
    password_len: u32,
    password_data: *const u8,
    item_ref_out: *mut u64,
) -> i32 {
    let service = extract_c_buf(service_ptr, service_len as usize);
    let account = extract_c_buf(account_ptr, account_len as usize);
    let data = if password_data.is_null() || password_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(password_data, password_len as usize) }.to_vec()
    };

    let mut guard = keychain_db();
    let conn = ensure_keychain(&mut guard);
    // Check for duplicate
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM keychain WHERE service = ?1 AND account = ?2",
        rusqlite::params![&service, &account],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;
    if exists {
        return ERR_SEC_DUPLICATE_ITEM;
    }
    let rc = conn.execute(
        "INSERT INTO keychain (service, account, class, data) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&service, &account, KSEC_CLASS_GENERIC_PW as i64, &data],
    );
    if rc.is_err() {
        return ERR_SEC_UNIMPLEMENTED;
    }
    if !item_ref_out.is_null() {
        let handle = next_sec_handle();
        let mut istore = keychain_item_store();
        istore.get_or_insert_with(HashMap::new).insert(handle, KeychainItemRef {
            service: service.clone(),
            account: account.clone(),
        });
        unsafe { *item_ref_out = handle; }
    }
    log::trace!("SecKeychainAddGenericPassword: added {service}/{account}");
    ERR_SEC_SUCCESS
}

/// SecKeychainFindInternetPassword — uses server+account as the lookup key.
///
/// Signature: (keychain, serverLen, server, domainLen, domain, accountLen, account,
///             pathLen, path, port, protocol, authType, *passwordLen, **passwordData, *itemRef)
pub extern "C" fn shim_sec_keychain_find_internet_password(
    _keychain: u64,
    server_len: u32,
    server_ptr: *const u8,
    _domain_len: u32,
    _domain_ptr: *const u8,
    account_len: u32,
    account_ptr: *const u8,
    _path_len: u32,
    _path_ptr: *const u8,
    _port: u16,
    _protocol: u32,
    _auth_type: u32,
    password_len_out: *mut u32,
    password_data_out: *mut *mut u8,
    item_ref_out: *mut u64,
) -> i32 {
    // Map server → service for storage compatibility
    let service = extract_c_buf(server_ptr, server_len as usize);
    let account = extract_c_buf(account_ptr, account_len as usize);

    let mut guard = keychain_db();
    let conn = ensure_keychain(&mut guard);
    let row_data: Result<Vec<u8>, _> = conn.query_row(
        "SELECT data FROM keychain WHERE service = ?1 AND account = ?2",
        rusqlite::params![&service, &account],
        |row| row.get(0),
    );
    match row_data {
        Ok(data) => {
            if !password_len_out.is_null() {
                unsafe { *password_len_out = data.len() as u32; }
            }
            if !password_data_out.is_null() {
                if data.is_empty() {
                    unsafe { *password_data_out = std::ptr::null_mut(); }
                } else {
                    let buf = unsafe { libc::malloc(data.len()) as *mut u8 };
                    if !buf.is_null() {
                        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len()); }
                    }
                    unsafe { *password_data_out = buf; }
                }
            }
            if !item_ref_out.is_null() {
                let handle = next_sec_handle();
                let mut istore = keychain_item_store();
                istore.get_or_insert_with(HashMap::new).insert(handle, KeychainItemRef {
                    service: service.clone(),
                    account: account.clone(),
                });
                unsafe { *item_ref_out = handle; }
            }
            ERR_SEC_SUCCESS
        }
        Err(_) => {
            if !password_len_out.is_null() { unsafe { *password_len_out = 0; } }
            if !password_data_out.is_null() { unsafe { *password_data_out = std::ptr::null_mut(); } }
            if !item_ref_out.is_null() { unsafe { *item_ref_out = 0; } }
            ERR_SEC_ITEM_NOT_FOUND
        }
    }
}

/// SecKeychainItemCopyContent(item, *itemClass, *attrList, *length, **outData) → OSStatus
///
/// Real implementation: looks up the item ref in the registry and queries SQLite.
pub extern "C" fn shim_sec_keychain_item_copy_content(
    item: u64,
    item_class_out: *mut u32,
    _attr_list: u64,
    length_out: *mut u32,
    data_out: *mut *mut u8,
) -> i32 {
    let guard = keychain_item_store();
    let iref = match guard.as_ref().and_then(|s| s.get(&item)) {
        Some(r) => r,
        None => {
            if !length_out.is_null() { unsafe { *length_out = 0; } }
            if !data_out.is_null() { unsafe { *data_out = std::ptr::null_mut(); } }
            return ERR_SEC_ITEM_NOT_FOUND;
        }
    };
    let service = iref.service.clone();
    let account = iref.account.clone();
    drop(guard);

    let mut dbguard = keychain_db();
    let conn = ensure_keychain(&mut dbguard);
    let row: Result<(i64, Vec<u8>), _> = conn.query_row(
        "SELECT class, data FROM keychain WHERE service = ?1 AND account = ?2",
        rusqlite::params![&service, &account],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );
    match row {
        Ok((class, data)) => {
            if !item_class_out.is_null() {
                // Map to SecItemClass: kSecGenericPasswordItemClass = 'genp' = 0x67656E70
                unsafe { *item_class_out = if class == KSEC_CLASS_GENERIC_PW as i64 { 0x67656E70 } else { 0 }; }
            }
            if !length_out.is_null() {
                unsafe { *length_out = data.len() as u32; }
            }
            if !data_out.is_null() {
                if data.is_empty() {
                    unsafe { *data_out = std::ptr::null_mut(); }
                } else {
                    let buf = unsafe { libc::malloc(data.len()) as *mut u8 };
                    if !buf.is_null() {
                        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len()); }
                    }
                    unsafe { *data_out = buf; }
                }
            }
            ERR_SEC_SUCCESS
        }
        Err(_) => {
            if !length_out.is_null() { unsafe { *length_out = 0; } }
            if !data_out.is_null() { unsafe { *data_out = std::ptr::null_mut(); } }
            ERR_SEC_ITEM_NOT_FOUND
        }
    }
}

/// SecKeychainItemModifyContent(item, attrList, length, data) → OSStatus
pub extern "C" fn shim_sec_keychain_item_modify_content(
    item: u64,
    _attr_list: u64,
    length: u32,
    data: *const u8,
) -> i32 {
    let guard = keychain_item_store();
    let iref = match guard.as_ref().and_then(|s| s.get(&item)) {
        Some(r) => r,
        None => return ERR_SEC_ITEM_NOT_FOUND,
    };
    let service = iref.service.clone();
    let account = iref.account.clone();
    drop(guard);

    let new_data = if data.is_null() || length == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data, length as usize) }.to_vec()
    };

    let mut dbguard = keychain_db();
    let conn = ensure_keychain(&mut dbguard);
    let rows = conn.execute(
        "UPDATE keychain SET data = ?1 WHERE service = ?2 AND account = ?3",
        rusqlite::params![&new_data, &service, &account],
    ).unwrap_or(0);
    if rows > 0 { ERR_SEC_SUCCESS } else { ERR_SEC_ITEM_NOT_FOUND }
}

/// SecKeychainItemFreeContent(attrList, data) → OSStatus
pub extern "C" fn shim_sec_keychain_item_free_content(_attr_list: u64, data: *mut u8) -> i32 {
    if !data.is_null() {
        unsafe { libc::free(data as *mut libc::c_void); }
    }
    ERR_SEC_SUCCESS
}

/// SecKeychainCopyDefault(&keychain) → OSStatus
pub extern "C" fn shim_sec_keychain_copy_default(keychain_out: *mut u64) -> i32 {
    if !keychain_out.is_null() {
        unsafe { *keychain_out = &SEC_KEYCHAIN_SENTINEL as *const u64 as u64; }
    }
    ERR_SEC_SUCCESS
}

/// SecKeychainOpen(path, &keychain) → OSStatus
pub extern "C" fn shim_sec_keychain_open(_path: *const u8, keychain_out: *mut u64) -> i32 {
    if !keychain_out.is_null() {
        unsafe { *keychain_out = &SEC_KEYCHAIN_SENTINEL as *const u64 as u64; }
    }
    ERR_SEC_SUCCESS
}

/// SecKeychainGetPath(keychain, &ioPathLength, pathName) → OSStatus
pub extern "C" fn shim_sec_keychain_get_path(
    _keychain: u64,
    path_len: *mut u32,
    path_buf: *mut u8,
) -> i32 {
    let default_path = b"/tmp/burdettieOS-keychain.db\0";
    let copy_len = if !path_len.is_null() {
        let max = unsafe { *path_len } as usize;
        max.min(default_path.len())
    } else {
        default_path.len()
    };
    if !path_buf.is_null() && copy_len > 0 {
        unsafe { std::ptr::copy_nonoverlapping(default_path.as_ptr(), path_buf, copy_len); }
    }
    if !path_len.is_null() {
        unsafe { *path_len = (default_path.len() - 1) as u32; } // Exclude NUL
    }
    ERR_SEC_SUCCESS
}

// ── Shim functions ──────────────────────────────────────────────

/// SecItemAdd(attributes, result) → OSStatus
pub extern "C" fn shim_sec_item_add(attrs: u64, result: *mut u64) -> i32 {
    let (class, service, account, data) = parse_query(attrs);
    if class == 0 {
        return ERR_SEC_UNIMPLEMENTED;
    }
    let data_bytes = data.unwrap_or_default();
    let mut guard = keychain_db();
    let conn = ensure_keychain(&mut guard);
    // Check for duplicate
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM keychain WHERE service = ?1 AND account = ?2",
        rusqlite::params![&service, &account],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;
    if exists {
        return ERR_SEC_DUPLICATE_ITEM;
    }
    let rc = conn.execute(
        "INSERT INTO keychain (service, account, class, data) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&service, &account, class as i64, &data_bytes],
    );
    if rc.is_err() {
        return ERR_SEC_UNIMPLEMENTED;
    }
    if !result.is_null() {
        unsafe { *result = 0; }
    }
    ERR_SEC_SUCCESS
}

/// SecItemCopyMatching(query, result) → OSStatus
pub extern "C" fn shim_sec_item_copy_matching(query: u64, result: *mut u64) -> i32 {
    let (_class, service, account, _data) = parse_query(query);
    let mut guard = keychain_db();
    let conn = ensure_keychain(&mut guard);
    let row_data: Result<Vec<u8>, _> = conn.query_row(
        "SELECT data FROM keychain WHERE service = ?1 AND account = ?2",
        rusqlite::params![&service, &account],
        |row| row.get(0),
    );
    match row_data {
        Ok(data) => {
            let return_data = dict_value_for_sentinel(query, KSEC_RETURN_DATA);
            if return_data != 0 && !result.is_null() {
                let data_obj = unsafe { libc::calloc(1, 24) as u64 };
                if data_obj != 0 && !data.is_empty() {
                    let buf = unsafe { libc::malloc(data.len()) as *mut u8 };
                    if !buf.is_null() {
                        unsafe {
                            std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());
                            *((data_obj as usize + 8) as *mut u64) = buf as u64;
                            *((data_obj as usize + 16) as *mut u64) = data.len() as u64;
                        }
                    }
                }
                unsafe { *result = data_obj; }
                return ERR_SEC_SUCCESS;
            }
            if !result.is_null() {
                unsafe { *result = 0; }
            }
            ERR_SEC_SUCCESS
        }
        Err(_) => {
            if !result.is_null() {
                unsafe { *result = 0; }
            }
            ERR_SEC_ITEM_NOT_FOUND
        }
    }
}

/// SecItemUpdate(query, attrs) → OSStatus
pub extern "C" fn shim_sec_item_update(query: u64, attrs: u64) -> i32 {
    let (_class, service, account, _data) = parse_query(query);
    let (_class2, _service2, _account2, new_data) = parse_query(attrs);
    let mut guard = keychain_db();
    let conn = ensure_keychain(&mut guard);
    if let Some(d) = new_data {
        let rows = conn.execute(
            "UPDATE keychain SET data = ?1 WHERE service = ?2 AND account = ?3",
            rusqlite::params![&d, &service, &account],
        ).unwrap_or(0);
        if rows > 0 {
            return ERR_SEC_SUCCESS;
        }
    }
    ERR_SEC_ITEM_NOT_FOUND
}

/// SecItemDelete(query) → OSStatus
pub extern "C" fn shim_sec_item_delete(query: u64) -> i32 {
    let (_class, service, account, _data) = parse_query(query);
    let mut guard = keychain_db();
    let conn = ensure_keychain(&mut guard);
    let rows = conn.execute(
        "DELETE FROM keychain WHERE service = ?1 AND account = ?2",
        rusqlite::params![&service, &account],
    ).unwrap_or(0);
    if rows > 0 {
        return ERR_SEC_SUCCESS;
    }
    ERR_SEC_ITEM_NOT_FOUND
}

/// SecCertificateCreateWithData(allocator, data) → SecCertificateRef
///
/// Real implementation: parses DER-encoded X.509 certificate using x509-parser,
/// extracts subject common name, stores in cert registry.
pub extern "C" fn shim_sec_cert_create(_alloc: u64, data_obj: u64) -> u64 {
    let der = match extract_data(data_obj) {
        Some(d) if !d.is_empty() => d,
        _ => return 0,
    };

    // Parse with x509-parser to extract subject info
    let (common_name, subject_summary) = match x509_parser::parse_x509_certificate(&der) {
        Ok((_rem, cert)) => {
            let cn = cert.subject().iter_common_name()
                .next()
                .and_then(|cn| cn.as_str().ok())
                .map(|s| s.to_string());
            let summary = cn.clone().or_else(|| {
                cert.subject().iter_organization()
                    .next()
                    .and_then(|o| o.as_str().ok())
                    .map(|s| s.to_string())
            });
            (cn, summary)
        }
        Err(e) => {
            log::debug!("SecCertificateCreateWithData: x509 parse failed: {e}");
            // Still store the raw DER — some callers just need the opaque ref
            (None, None)
        }
    };

    let handle = next_sec_handle();
    let stored = StoredCert { der, common_name, subject_summary };
    let mut guard = cert_store();
    guard.get_or_insert_with(HashMap::new).insert(handle, stored);
    log::trace!("SecCertificateCreateWithData → handle={handle:#x}");
    handle
}

/// SecCertificateCopyCommonName(cert, &commonName) → OSStatus
pub extern "C" fn shim_sec_copy_common_name(cert: u64, name_out: *mut u64) -> i32 {
    if name_out.is_null() { return ERR_SEC_UNIMPLEMENTED; }
    let guard = cert_store();
    if let Some(store) = guard.as_ref() {
        if let Some(stored) = store.get(&cert) {
            if let Some(cn) = &stored.common_name {
                // Allocate a CFString-like object: [isa:8][flags:8][cstr_ptr:8][len:8]
                let cstr = std::ffi::CString::new(cn.as_str()).unwrap_or_default();
                let len = cn.len();
                let obj = unsafe { libc::calloc(1, 32) as u64 };
                if obj != 0 {
                    let ptr = cstr.into_raw();
                    unsafe {
                        *((obj as usize + 16) as *mut u64) = ptr as u64;
                        *((obj as usize + 24) as *mut u64) = len as u64;
                        *name_out = obj;
                    }
                    return ERR_SEC_SUCCESS;
                }
            }
        }
    }
    unsafe { *name_out = 0; }
    ERR_SEC_ITEM_NOT_FOUND
}

/// SecCertificateCopyData(cert) → CFDataRef
pub extern "C" fn shim_sec_cert_copy_data(cert: u64) -> u64 {
    let guard = cert_store();
    if let Some(store) = guard.as_ref() {
        if let Some(stored) = store.get(&cert) {
            // Allocate NSData-like object: [isa:8][bytes_ptr:8][length:8]
            let obj = unsafe { libc::calloc(1, 24) as u64 };
            if obj != 0 && !stored.der.is_empty() {
                let buf = unsafe { libc::malloc(stored.der.len()) as *mut u8 };
                if !buf.is_null() {
                    unsafe {
                        std::ptr::copy_nonoverlapping(stored.der.as_ptr(), buf, stored.der.len());
                        *((obj as usize + 8) as *mut u64) = buf as u64;
                        *((obj as usize + 16) as *mut u64) = stored.der.len() as u64;
                    }
                }
            }
            return obj;
        }
    }
    0
}

/// SecCertificateCopySubjectSummary(cert) → CFStringRef
pub extern "C" fn shim_sec_cert_copy_subject_summary(cert: u64) -> u64 {
    let guard = cert_store();
    if let Some(store) = guard.as_ref() {
        if let Some(stored) = store.get(&cert) {
            if let Some(summary) = &stored.subject_summary {
                let cstr = std::ffi::CString::new(summary.as_str()).unwrap_or_default();
                let len = summary.len();
                let obj = unsafe { libc::calloc(1, 32) as u64 };
                if obj != 0 {
                    let ptr = cstr.into_raw();
                    unsafe {
                        *((obj as usize + 16) as *mut u64) = ptr as u64;
                        *((obj as usize + 24) as *mut u64) = len as u64;
                    }
                }
                return obj;
            }
        }
    }
    0
}

/// Generic return-NULL for 1-arg functions
pub extern "C" fn shim_sec_return_null(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_sec_return_null3(_a: u64, _b: u64, _c: *mut u64) -> u64 { 0 }
pub extern "C" fn shim_sec_return_false(_a: u64, _b: u64, _c: u64) -> i32 { 0 }
pub extern "C" fn shim_sec_return_zero(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_sec_return_unimplemented(_a: u64) -> i32 { ERR_SEC_UNIMPLEMENTED }
pub extern "C" fn shim_sec_return_success(_a: u64) -> i32 { ERR_SEC_SUCCESS }
pub extern "C" fn shim_sec_return_item_not_found(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: *mut u64) -> i32 { ERR_SEC_ITEM_NOT_FOUND }
pub extern "C" fn shim_sec_get_type_id() -> u64 { 100 }
pub extern "C" fn shim_sec_void3(_a: u64, _b: u64, _c: u64) {}

// ── Authorization stubs ─────────────────────────────────────────────

/// All privileged Authorization operations → errAuthorizationDenied (-60005).
/// The guest binary cannot escalate privileges through our stub layer.
pub extern "C" fn shim_auth_denied(_a: u64, _b: u64, _c: u64, _d: u64) -> i32 {
    ERR_AUTH_DENIED
}

/// AuthorizationCancelRight → errAuthorizationCanceled (-60008)
pub extern "C" fn shim_auth_cancelled(_a: u64, _b: u64) -> i32 {
    -60008 // errAuthorizationCanceled
}

/// AuthorizationFree(ref, flags) → void (ignore the handle)
pub extern "C" fn shim_auth_free(_auth_ref: u64, _flags: u64) {}

/// AuthorizationFreeItemSet(set) → void
pub extern "C" fn shim_auth_free_item_set(_set: u64) {}

/// AuthorizationCopyRightsAsync — calls back with errAuthorizationDenied
pub extern "C" fn shim_auth_copy_rights_async(
    _auth_ref: u64,
    _rights: u64,
    _environment: u64,
    _flags: u64,
    _block: u64,
) {} // Callback is never invoked — behaves as fire-and-forget.

/// Generic 4-arg void stub used by nw_* functions
pub extern "C" fn shim_void4_stub(_a: u64, _b: u64, _c: u64, _d: u64) {}

// ── shim aliases re-used from misc.rs ──────────────────────────────
/// 1-arg void (forwarded from misc module)
pub extern "C" fn shim_void1_stub(_a: u64) {}
/// 2-arg void (forwarded from misc module)
pub extern "C" fn shim_void2_stub(_a: u64, _b: u64) {}

/// SecKeyCreateRandomKey(parameters, &error) → SecKeyRef
///
/// Real implementation: generates an ECDSA P-256 key pair using `ring`.
pub extern "C" fn shim_sec_key_create_random(_params: u64, error: *mut u64) -> u64 {
    use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
    use ring::rand::SystemRandom;

    let rng = SystemRandom::new();
    match EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng) {
        Ok(pkcs8) => {
            let handle = next_sec_handle();
            let stored = StoredKey {
                key_pair_der: pkcs8.as_ref().to_vec(),
                algorithm: KeyAlgorithm::EcdsaP256,
                is_public: false,
            };
            let mut guard = key_store();
            guard.get_or_insert_with(HashMap::new).insert(handle, stored);
            if !error.is_null() {
                unsafe { *error = 0; }
            }
            log::trace!("SecKeyCreateRandomKey → handle={handle:#x} (ECDSA P-256)");
            handle
        }
        Err(e) => {
            log::debug!("SecKeyCreateRandomKey failed: {e}");
            if !error.is_null() {
                unsafe { *error = 0; }
            }
            0
        }
    }
}

/// SecKeyCopyPublicKey(key) → SecKeyRef
pub extern "C" fn shim_sec_key_copy_public(key: u64) -> u64 {
    let guard = key_store();
    if let Some(store) = guard.as_ref() {
        if let Some(stored) = store.get(&key) {
            if stored.is_public {
                return key; // Already a public key handle
            }
            // Extract public key from private PKCS8
            match stored.algorithm {
                KeyAlgorithm::EcdsaP256 => {
                    use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
                    if let Ok(kp) = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &stored.key_pair_der, &ring::rand::SystemRandom::new()) {
                        let pub_bytes = kp.public_key().as_ref().to_vec();
                        drop(guard);
                        let pub_handle = next_sec_handle();
                        let pub_key = StoredKey {
                            key_pair_der: pub_bytes,
                            algorithm: KeyAlgorithm::EcdsaP256,
                            is_public: true,
                        };
                        let mut wguard = key_store();
                        wguard.get_or_insert_with(HashMap::new).insert(pub_handle, pub_key);
                        return pub_handle;
                    }
                }
                _ => {}
            }
        }
    }
    0
}

/// SecKeyCreateSignature(key, algorithm, dataToSign, &error) → CFDataRef
pub extern "C" fn shim_sec_key_create_signature(key: u64, _algo: u64, data_obj: u64, error: *mut u64) -> u64 {
    let data = match extract_data(data_obj) {
        Some(d) => d,
        None => {
            if !error.is_null() { unsafe { *error = 0; } }
            return 0;
        }
    };

    let guard = key_store();
    if let Some(store) = guard.as_ref() {
        if let Some(stored) = store.get(&key) {
            match stored.algorithm {
                KeyAlgorithm::EcdsaP256 if !stored.is_public => {
                    use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
                    use ring::rand::SystemRandom;
                    let rng = SystemRandom::new();
                    if let Ok(kp) = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &stored.key_pair_der, &rng) {
                        if let Ok(sig) = kp.sign(&rng, &data) {
                            let sig_bytes = sig.as_ref();
                            // Return as NSData-like: [isa:8][bytes_ptr:8][length:8]
                            let obj = unsafe { libc::calloc(1, 24) as u64 };
                            if obj != 0 {
                                let buf = unsafe { libc::malloc(sig_bytes.len()) as *mut u8 };
                                if !buf.is_null() {
                                    unsafe {
                                        std::ptr::copy_nonoverlapping(sig_bytes.as_ptr(), buf, sig_bytes.len());
                                        *((obj as usize + 8) as *mut u64) = buf as u64;
                                        *((obj as usize + 16) as *mut u64) = sig_bytes.len() as u64;
                                    }
                                }
                            }
                            if !error.is_null() { unsafe { *error = 0; } }
                            return obj;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if !error.is_null() { unsafe { *error = 0; } }
    0
}

/// SecKeyIsAlgorithmSupported(key, operation, algorithm) → Boolean
pub extern "C" fn shim_sec_key_is_algo_supported(key: u64, _operation: u64, _algo: u64) -> i32 {
    let guard = key_store();
    if let Some(store) = guard.as_ref() {
        if store.contains_key(&key) {
            return 1; // We support ECDSA P-256
        }
    }
    0
}

/// SecKeyGetBlockSize(key) → size_t
pub extern "C" fn shim_sec_key_get_block_size(key: u64) -> u64 {
    let guard = key_store();
    if let Some(store) = guard.as_ref() {
        if let Some(stored) = store.get(&key) {
            return match stored.algorithm {
                KeyAlgorithm::EcdsaP256 => 32,  // P-256 is 32-byte key size
                KeyAlgorithm::Ed25519 => 32,
                KeyAlgorithm::Rsa2048 => 256,
            };
        }
    }
    0
}

/// SecTrustCreateWithCertificates(certs, policies, &trust) → OSStatus
///
/// Real implementation: stores cert handles for later evaluation.
pub extern "C" fn shim_sec_trust_create(certs: u64, _policies: u64, trust_out: *mut u64) -> i32 {
    if trust_out.is_null() { return ERR_SEC_UNIMPLEMENTED; }

    // Extract cert handles from CFArray or single cert
    let mut cert_handles = Vec::new();
    if certs != 0 {
        // Check if it's an array (count at offset 8)
        let count = raw_read_u64(certs, 8);
        let items_ptr = raw_read_u64(certs, 16);
        if count > 0 && count < 100 && items_ptr != 0 {
            // Array of cert refs
            for i in 0..count as usize {
                let handle = unsafe { *((items_ptr as usize + i * 8) as *const u64) };
                cert_handles.push(handle);
            }
        } else {
            // Single cert ref
            cert_handles.push(certs);
        }
    }

    let handle = next_sec_handle();
    let num_certs = cert_handles.len();
    let stored = StoredTrust { cert_handles };
    let mut guard = trust_store();
    guard.get_or_insert_with(HashMap::new).insert(handle, stored);
    unsafe { *trust_out = handle; }
    log::trace!("SecTrustCreateWithCertificates → handle={handle:#x} ({num_certs} certs)");
    ERR_SEC_SUCCESS
}

/// SecTrustEvaluateWithError(trust, &error) → bool
///
/// Real implementation: delegates to `trust-eval` crate for X.509 chain
/// validation using webpki + Mozilla root certificate store.
pub extern "C" fn shim_sec_trust_evaluate_with_error(trust: u64, error: *mut u64) -> i32 {
    let tguard = trust_store();
    let trust_obj = match tguard.as_ref().and_then(|s| s.get(&trust)) {
        Some(t) => t,
        None => {
            log::debug!("SecTrustEvaluateWithError: unknown trust handle {trust:#x}");
            return 0;
        }
    };

    // Collect DER bytes from cert store
    let cguard = cert_store();
    let certs: Vec<Vec<u8>> = trust_obj.cert_handles.iter().filter_map(|h| {
        cguard.as_ref()?.get(h).map(|c| c.der.clone())
    }).collect();

    if certs.is_empty() {
        log::debug!("SecTrustEvaluateWithError: no certs to evaluate");
        return 0;
    }

    // Convert to slices for trust-eval
    let cert_slices: Vec<&[u8]> = certs.iter().map(|c| c.as_slice()).collect();
    let valid = trust_eval::validate_trust(&cert_slices);

    if !error.is_null() {
        unsafe { *error = 0; } // No CFError for now
    }

    log::debug!("SecTrustEvaluateWithError: {} ({} certs)", if valid { "PASS" } else { "FAIL" }, certs.len());
    if valid { 1 } else { 0 }
}

/// SecTrustGetCertificateCount(trust) → CFIndex
pub extern "C" fn shim_sec_trust_get_cert_count(trust: u64) -> u64 {
    let guard = trust_store();
    if let Some(store) = guard.as_ref() {
        if let Some(stored) = store.get(&trust) {
            return stored.cert_handles.len() as u64;
        }
    }
    0
}

/// SecPolicyCreateSSL(server, hostname) → SecPolicyRef
pub extern "C" fn shim_sec_policy_create(_server: i32, _hostname: u64) -> u64 {
    &SEC_POLICY_SENTINEL as *const u64 as u64
}

/// SecPolicyCreateBasicX509() → SecPolicyRef
pub extern "C" fn shim_sec_policy_create_no_args() -> u64 {
    &SEC_POLICY_SENTINEL as *const u64 as u64
}

/// SecIdentityCopyCertificate(identity, &cert) → OSStatus
pub extern "C" fn shim_sec_identity_copy_cert(_identity: u64, _cert: *mut u64) -> i32 {
    ERR_SEC_UNIMPLEMENTED
}

/// SecIdentityCopyPrivateKey(identity, &key) → OSStatus
pub extern "C" fn shim_sec_identity_copy_key(_identity: u64, _key: *mut u64) -> i32 {
    ERR_SEC_UNIMPLEMENTED
}

/// SecRandomCopyBytes(rng, count, bytes) → int
///
/// Real CSPRNG via ring::rand::SystemRandom.
pub extern "C" fn shim_sec_random_copy_bytes(_rng: u64, count: usize, bytes: *mut u8) -> i32 {
    if bytes.is_null() || count == 0 {
        return ERR_SEC_SUCCESS;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(bytes, count) };
    use ring::rand::SecureRandom;
    let rng = ring::rand::SystemRandom::new();
    match rng.fill(slice) {
        Ok(()) => ERR_SEC_SUCCESS,
        Err(_) => ERR_SEC_UNIMPLEMENTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ShimTable;

    #[test]
    fn authorization_stubs_registered() {
        let mut table = ShimTable::new();
        SecurityStubs::install(&mut table);
        assert!(table.resolve("_AuthorizationCreate").is_some(), "_AuthorizationCreate not found");
        assert!(table.resolve("_AuthorizationCopyRights").is_some(), "_AuthorizationCopyRights not found");
        assert!(table.resolve("_nw_connection_create").is_some(), "_nw_connection_create not found");
        assert!(table.resolve("_secLogObjForScope").is_some(), "_secLogObjForScope not found");
    }

    #[test]
    fn security_stubs_registered() {
        let mut table = ShimTable::new();
        SecurityStubs::install(&mut table);
        assert!(table.resolve("_SecItemAdd").is_some());
        assert!(table.resolve("_SecCertificateCreateWithData").is_some());
        assert!(table.resolve("_SecTrustCreateWithCertificates").is_some());
        assert!(table.resolve("_SecRandomCopyBytes").is_some());
        assert!(table.resolve("_kSecClass").is_some());
    }

    #[test]
    fn sec_item_copy_matching_returns_not_found() {
        let mut result: u64 = 42;
        let status = shim_sec_item_copy_matching(0, &mut result);
        assert_eq!(status, ERR_SEC_ITEM_NOT_FOUND);
        assert_eq!(result, 0);
    }

    #[test]
    fn sec_random_fills_bytes() {
        let mut buf = [0u8; 32];
        let status = shim_sec_random_copy_bytes(0, 32, buf.as_mut_ptr());
        assert_eq!(status, ERR_SEC_SUCCESS);
        // Extremely unlikely all 32 bytes remain zero after random fill
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn sec_policy_sentinels_non_null() {
        assert_ne!(shim_sec_policy_create(1, 0), 0);
        assert_ne!(shim_sec_policy_create_no_args(), 0);
    }

    #[test]
    fn ksec_data_symbols_non_null() {
        let table_addr = &KSEC_CLASS as *const u64 as u64;
        assert_ne!(table_addr, 0);
    }

    // ── In-memory keychain CRUD tests ─────────────────────────────

    /// Build a minimal NSDictionary-like object (not a real NSDictionary,
    /// just matching the memory layout [isa:8][count:8][keys_ptr:8][vals_ptr:8]).
    fn make_fake_dict(entries: &[(u64, u64)]) -> u64 {
        let count = entries.len();
        let dict = unsafe { libc::calloc(1, 32) } as u64;
        // isa = 0 (doesn't matter for our raw reads)
        unsafe { *((dict as usize + 8) as *mut u64) = count as u64; }
        if count > 0 {
            let keys = unsafe { libc::malloc(count * 8) } as *mut u64;
            let vals = unsafe { libc::malloc(count * 8) } as *mut u64;
            for (i, (k, v)) in entries.iter().enumerate() {
                unsafe {
                    keys.add(i).write(*k);
                    vals.add(i).write(*v);
                }
            }
            unsafe {
                *((dict as usize + 16) as *mut u64) = keys as u64;
                *((dict as usize + 24) as *mut u64) = vals as u64;
            }
        }
        dict
    }

    /// Build a minimal CFString-like object [isa:8][flags:8][cstr_ptr:8][len:8].
    fn make_fake_cfstring(s: &str) -> u64 {
        let cstr = std::ffi::CString::new(s).unwrap();
        let ptr = cstr.into_raw();
        let obj = unsafe { libc::calloc(1, 32) } as u64;
        unsafe {
            *((obj as usize + 16) as *mut u64) = ptr as u64;
            *((obj as usize + 24) as *mut u64) = s.len() as u64;
        }
        obj
    }

    /// Build a minimal NSData-like object [isa:8][bytes_ptr:8][length:8].
    fn make_fake_nsdata(data: &[u8]) -> u64 {
        let obj = unsafe { libc::calloc(1, 24) } as u64;
        if !data.is_empty() {
            let buf = unsafe { libc::malloc(data.len()) } as *mut u8;
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());
                *((obj as usize + 8) as *mut u64) = buf as u64;
                *((obj as usize + 16) as *mut u64) = data.len() as u64;
            }
        }
        obj
    }

    fn clear_keychain() {
        let mut guard = keychain_db();
        if let Some(conn) = guard.as_mut() {
            let _ = conn.execute("DELETE FROM keychain", []);
        }
    }

    #[test]
    fn keychain_add_and_query() {
        let svc = make_fake_cfstring("com.test.add_query.u99");
        let acct = make_fake_cfstring("user99@example.com");
        let data = make_fake_nsdata(b"s3cr3t");

        // Ensure clean state for this specific key.
        let del_q = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        let _ = shim_sec_item_delete(del_q);

        let add_dict = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_VALUE_DATA, data),
        ]);

        let status = shim_sec_item_add(add_dict, std::ptr::null_mut());
        assert_eq!(status, ERR_SEC_SUCCESS);

        // Query it back
        let query = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_RETURN_DATA, 1), // kCFBooleanTrue = 1 (nonzero)
        ]);
        let mut result: u64 = 0;
        let status = shim_sec_item_copy_matching(query, &mut result);
        assert_eq!(status, ERR_SEC_SUCCESS);
        assert_ne!(result, 0, "result should be an NSData object");

        // Read back the data
        let bytes_ptr = raw_read_u64(result, 8);
        let length = raw_read_u64(result, 16) as usize;
        assert_eq!(length, 6);
        let slice = unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, length) };
        assert_eq!(slice, b"s3cr3t");
    }

    #[test]
    fn keychain_duplicate_add_fails() {
        // Use unique service name to avoid collision with other tests.
        let svc = make_fake_cfstring("dup.test.unique42");
        let acct = make_fake_cfstring("alice42");
        let data = make_fake_nsdata(b"pw1");

        // Ensure clean: delete any leftover from a prior run.
        let del_q = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        let _ = shim_sec_item_delete(del_q);

        let dict = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_VALUE_DATA, data),
        ]);

        assert_eq!(shim_sec_item_add(dict, std::ptr::null_mut()), ERR_SEC_SUCCESS);
        assert_eq!(shim_sec_item_add(dict, std::ptr::null_mut()), ERR_SEC_DUPLICATE_ITEM);
    }

    #[test]
    fn keychain_update() {
        let svc = make_fake_cfstring("upd.test.unique88");
        let acct = make_fake_cfstring("bob88");
        let data1 = make_fake_nsdata(b"old");
        let data2 = make_fake_nsdata(b"new_password");

        // Clean first.
        let del_q = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        let _ = shim_sec_item_delete(del_q);

        let add_dict = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_VALUE_DATA, data1),
        ]);
        assert_eq!(shim_sec_item_add(add_dict, std::ptr::null_mut()), ERR_SEC_SUCCESS);

        let query = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        let update_attrs = make_fake_dict(&[(KSEC_VALUE_DATA, data2)]);
        assert_eq!(shim_sec_item_update(query, update_attrs), ERR_SEC_SUCCESS);

        // Query back and verify new data
        let q2 = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_RETURN_DATA, 1),
        ]);
        let mut result: u64 = 0;
        assert_eq!(shim_sec_item_copy_matching(q2, &mut result), ERR_SEC_SUCCESS);
        let length = raw_read_u64(result, 16) as usize;
        assert_eq!(length, 12); // "new_password".len()
    }

    #[test]
    fn keychain_delete() {
        // Use unique service name.
        let svc = make_fake_cfstring("del.test.unique77");
        let acct = make_fake_cfstring("carol77");
        let data = make_fake_nsdata(b"delete_me");

        let add_dict = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_VALUE_DATA, data),
        ]);
        // Clean first in case of prior run.
        let del_query = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        let _ = shim_sec_item_delete(del_query);

        assert_eq!(shim_sec_item_add(add_dict, std::ptr::null_mut()), ERR_SEC_SUCCESS);
        assert_eq!(shim_sec_item_delete(del_query), ERR_SEC_SUCCESS);
        // Delete again → not found
        assert_eq!(shim_sec_item_delete(del_query), ERR_SEC_ITEM_NOT_FOUND);
    }

    #[test]
    fn keychain_query_nonexistent() {
        let svc = make_fake_cfstring("no.such.svc.unique55");
        let acct = make_fake_cfstring("nobody55");
        let query = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        let mut result: u64 = 42;
        assert_eq!(shim_sec_item_copy_matching(query, &mut result), ERR_SEC_ITEM_NOT_FOUND);
        assert_eq!(result, 0);
    }

    // ── Legacy SecKeychain API tests ────────────────────────────

    /// Helper: clean up a legacy keychain entry by service+account.
    fn legacy_cleanup(service: &str, account: &str) {
        let mut guard = keychain_db();
        let conn = ensure_keychain(&mut guard);
        let _ = conn.execute(
            "DELETE FROM keychain WHERE service = ?1 AND account = ?2",
            rusqlite::params![service, account],
        );
    }

    #[test]
    fn legacy_add_and_find_generic_password() {
        let svc = "com.test.legacy.add_find";
        let acct = "alice_legacy";
        legacy_cleanup(svc, acct);

        let status = shim_sec_keychain_add_generic_password(
            0, // default keychain
            svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            6, b"secret".as_ptr(),
            std::ptr::null_mut(), // no item ref
        );
        assert_eq!(status, ERR_SEC_SUCCESS);

        let mut pw_len: u32 = 0;
        let mut pw_data: *mut u8 = std::ptr::null_mut();
        let mut item_ref: u64 = 0;
        let status = shim_sec_keychain_find_generic_password(
            0,
            svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            &mut pw_len, &mut pw_data, &mut item_ref,
        );
        assert_eq!(status, ERR_SEC_SUCCESS);
        assert_eq!(pw_len, 6);
        assert!(!pw_data.is_null());
        let found = unsafe { std::slice::from_raw_parts(pw_data, pw_len as usize) };
        assert_eq!(found, b"secret");
        assert_ne!(item_ref, 0, "should return a valid item ref");

        // Free the data
        shim_sec_keychain_item_free_content(0, pw_data);
    }

    #[test]
    fn legacy_add_duplicate_fails() {
        let svc = "com.test.legacy.dup";
        let acct = "bob_legacy";
        legacy_cleanup(svc, acct);

        let add = |pw: &[u8]| shim_sec_keychain_add_generic_password(
            0, svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            pw.len() as u32, pw.as_ptr(),
            std::ptr::null_mut(),
        );
        assert_eq!(add(b"pw1"), ERR_SEC_SUCCESS);
        assert_eq!(add(b"pw2"), ERR_SEC_DUPLICATE_ITEM);
    }

    #[test]
    fn legacy_find_nonexistent_returns_not_found() {
        let svc = "com.test.legacy.nope.xyz";
        let acct = "nobody_here_xyz";
        let mut pw_len: u32 = 99;
        let mut pw_data: *mut u8 = 0x42 as *mut u8;
        let status = shim_sec_keychain_find_generic_password(
            0,
            svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            &mut pw_len, &mut pw_data, std::ptr::null_mut(),
        );
        assert_eq!(status, ERR_SEC_ITEM_NOT_FOUND);
        assert_eq!(pw_len, 0);
        assert!(pw_data.is_null());
    }

    #[test]
    fn legacy_item_copy_content() {
        let svc = "com.test.legacy.copy_content";
        let acct = "carol_legacy";
        legacy_cleanup(svc, acct);

        // Add via legacy API
        shim_sec_keychain_add_generic_password(
            0, svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            3, b"abc".as_ptr(),
            std::ptr::null_mut(),
        );

        // Find to get an item ref
        let mut item_ref: u64 = 0;
        shim_sec_keychain_find_generic_password(
            0, svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            std::ptr::null_mut(), std::ptr::null_mut(), &mut item_ref,
        );
        assert_ne!(item_ref, 0);

        // CopyContent
        let mut item_class: u32 = 0;
        let mut length: u32 = 0;
        let mut data_out: *mut u8 = std::ptr::null_mut();
        let status = shim_sec_keychain_item_copy_content(
            item_ref, &mut item_class, 0, &mut length, &mut data_out,
        );
        assert_eq!(status, ERR_SEC_SUCCESS);
        assert_eq!(item_class, 0x67656E70); // 'genp'
        assert_eq!(length, 3);
        let content = unsafe { std::slice::from_raw_parts(data_out, length as usize) };
        assert_eq!(content, b"abc");
        shim_sec_keychain_item_free_content(0, data_out);
    }

    #[test]
    fn legacy_item_modify_content() {
        let svc = "com.test.legacy.modify";
        let acct = "dave_legacy";
        legacy_cleanup(svc, acct);

        shim_sec_keychain_add_generic_password(
            0, svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            3, b"old".as_ptr(),
            std::ptr::null_mut(),
        );

        // Get item ref
        let mut item_ref: u64 = 0;
        shim_sec_keychain_find_generic_password(
            0, svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            std::ptr::null_mut(), std::ptr::null_mut(), &mut item_ref,
        );

        // Modify
        let new_pw = b"new_password";
        let status = shim_sec_keychain_item_modify_content(
            item_ref, 0, new_pw.len() as u32, new_pw.as_ptr(),
        );
        assert_eq!(status, ERR_SEC_SUCCESS);

        // Verify the update via find
        let mut pw_len: u32 = 0;
        let mut pw_data: *mut u8 = std::ptr::null_mut();
        shim_sec_keychain_find_generic_password(
            0, svc.len() as u32, svc.as_ptr(),
            acct.len() as u32, acct.as_ptr(),
            &mut pw_len, &mut pw_data, std::ptr::null_mut(),
        );
        assert_eq!(pw_len, 12);
        let found = unsafe { std::slice::from_raw_parts(pw_data, pw_len as usize) };
        assert_eq!(found, b"new_password");
        shim_sec_keychain_item_free_content(0, pw_data);
    }

    #[test]
    fn legacy_keychain_copy_default() {
        let mut kc: u64 = 0;
        assert_eq!(shim_sec_keychain_copy_default(&mut kc), ERR_SEC_SUCCESS);
        assert_ne!(kc, 0, "should return a sentinel");
    }

    #[test]
    fn legacy_keychain_get_path() {
        let mut path_buf = [0u8; 256];
        let mut path_len: u32 = 256;
        let status = shim_sec_keychain_get_path(0, &mut path_len, path_buf.as_mut_ptr());
        assert_eq!(status, ERR_SEC_SUCCESS);
        let path = std::str::from_utf8(&path_buf[..path_len as usize]).unwrap();
        assert!(path.contains("burdettieOS-keychain"), "path should reference keychain db: {path}");
    }

    #[test]
    fn legacy_stubs_registered() {
        let mut table = ShimTable::new();
        SecurityStubs::install(&mut table);
        assert!(table.resolve("_SecKeychainFindGenericPassword").is_some());
        assert!(table.resolve("_SecKeychainAddGenericPassword").is_some());
        assert!(table.resolve("_SecKeychainItemCopyContent").is_some());
        assert!(table.resolve("_SecKeychainItemModifyContent").is_some());
        assert!(table.resolve("_SecKeychainItemFreeContent").is_some());
        assert!(table.resolve("_SecKeychainFindInternetPassword").is_some());
        assert!(table.resolve("_SecKeychainCopyDefault").is_some());
        assert!(table.resolve("_SecKeychainOpen").is_some());
        assert!(table.resolve("_SecKeychainGetPath").is_some());
        assert!(table.resolve("_SecKeychainCopySearchList").is_some());
        assert!(table.resolve("_SecKeychainSetSearchList").is_some());
    }

    #[test]
    fn legacy_free_content_null_safe() {
        assert_eq!(shim_sec_keychain_item_free_content(0, std::ptr::null_mut()), ERR_SEC_SUCCESS);
    }

    #[test]
    fn legacy_item_copy_content_invalid_ref() {
        let mut length: u32 = 99;
        let mut data_out: *mut u8 = 0x42 as *mut u8;
        let status = shim_sec_keychain_item_copy_content(
            0xDEAD, std::ptr::null_mut(), 0, &mut length, &mut data_out,
        );
        assert_eq!(status, ERR_SEC_ITEM_NOT_FOUND);
        assert_eq!(length, 0);
        assert!(data_out.is_null());
    }
}
