//! Apple Security.framework stubs.
//!
//! Provides an in-memory keychain for SecItem CRUD, plus safe defaults
//! for certificate, key, trust, policy, and authorization APIs.
//! The keychain stores generic password items keyed by (service, account).

use crate::table::ShimTable;
use std::collections::HashMap;
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
            ("_SecItemAdd", shim_sec_item_add as u64, "SecItemAdd (stub → errSecUnimplemented)"),
            ("_SecItemCopyMatching", shim_sec_item_copy_matching as u64, "SecItemCopyMatching (stub → errSecItemNotFound)"),
            ("_SecItemUpdate", shim_sec_item_update as u64, "SecItemUpdate (stub → errSecUnimplemented)"),
            ("_SecItemDelete", shim_sec_item_delete as u64, "SecItemDelete (stub → errSecUnimplemented)")
        );

        // ── SecCertificate ──────────────────────────────────────
        register_shims!(table,
            ("_SecCertificateCreateWithData", shim_sec_cert_create as u64, "SecCertificateCreateWithData (stub → NULL)"),
            ("_SecCertificateCopyData", shim_sec_return_null as u64, "SecCertificateCopyData (stub → NULL)"),
            ("_SecCertificateCopySubjectSummary", shim_sec_return_null as u64, "SecCertificateCopySubjectSummary (stub → NULL)"),
            ("_SecCertificateCopyCommonName", shim_sec_copy_common_name as u64, "SecCertificateCopyCommonName (stub → errSecUnimplemented)"),
            ("_SecCertificateGetTypeID", shim_sec_get_type_id as u64, "SecCertificateGetTypeID (stub → 100)")
        );

        // ── SecKey ──────────────────────────────────────────────
        register_shims!(table,
            ("_SecKeyCreateRandomKey", shim_sec_key_create_random as u64, "SecKeyCreateRandomKey (stub → NULL)"),
            ("_SecKeyCopyPublicKey", shim_sec_return_null as u64, "SecKeyCopyPublicKey (stub → NULL)"),
            ("_SecKeyCopyExternalRepresentation", shim_sec_return_null as u64, "SecKeyCopyExternalRepresentation (stub → NULL)"),
            ("_SecKeyCreateSignature", shim_sec_return_null as u64, "SecKeyCreateSignature (stub → NULL)"),
            ("_SecKeyCreateEncryptedData", shim_sec_return_null as u64, "SecKeyCreateEncryptedData (stub → NULL)"),
            ("_SecKeyCreateDecryptedData", shim_sec_return_null as u64, "SecKeyCreateDecryptedData (stub → NULL)"),
            ("_SecKeyGetTypeID", shim_sec_get_type_id as u64, "SecKeyGetTypeID (stub → 100)"),
            ("_SecKeyIsAlgorithmSupported", shim_sec_return_false as u64, "SecKeyIsAlgorithmSupported (stub → false)"),
            ("_SecKeyGetBlockSize", shim_sec_return_zero as u64, "SecKeyGetBlockSize (stub → 0)")
        );

        // ── SecTrust ────────────────────────────────────────────
        register_shims!(table,
            ("_SecTrustCreateWithCertificates", shim_sec_trust_create as u64, "SecTrustCreateWithCertificates (stub → errSecUnimplemented)"),
            ("_SecTrustEvaluateWithError", shim_sec_trust_evaluate_with_error as u64, "SecTrustEvaluateWithError (stub → false/fail)"),
            ("_SecTrustEvaluateAsyncWithError", shim_sec_void3 as u64, "SecTrustEvaluateAsyncWithError (stub → void)"),
            ("_SecTrustGetCertificateCount", shim_sec_return_zero as u64, "SecTrustGetCertificateCount (stub → 0)"),
            ("_SecTrustCopyResult", shim_sec_return_null as u64, "SecTrustCopyResult (stub → NULL)"),
            ("_SecTrustSetPolicies", shim_sec_return_unimplemented as u64, "SecTrustSetPolicies (stub → errSecUnimplemented)"),
            ("_SecTrustSetAnchorCertificates", shim_sec_return_unimplemented as u64, "SecTrustSetAnchorCertificates (stub → errSecUnimplemented)"),
            ("_SecTrustGetTypeID", shim_sec_get_type_id as u64, "SecTrustGetTypeID (stub → 100)")
        );

        // ── SecPolicy ───────────────────────────────────────────
        register_shims!(table,
            ("_SecPolicyCreateSSL", shim_sec_policy_create as u64, "SecPolicyCreateSSL (stub → sentinel)"),
            ("_SecPolicyCreateBasicX509", shim_sec_policy_create_no_args as u64, "SecPolicyCreateBasicX509 (stub → sentinel)"),
            ("_SecPolicyGetTypeID", shim_sec_get_type_id as u64, "SecPolicyGetTypeID (stub → 100)")
        );

        // ── SecIdentity ─────────────────────────────────────────
        register_shims!(table,
            ("_SecIdentityGetTypeID", shim_sec_get_type_id as u64, "SecIdentityGetTypeID (stub → 100)"),
            ("_SecIdentityCopyCertificate", shim_sec_identity_copy_cert as u64, "SecIdentityCopyCertificate (stub → errSecUnimplemented)"),
            ("_SecIdentityCopyPrivateKey", shim_sec_identity_copy_key as u64, "SecIdentityCopyPrivateKey (stub → errSecUnimplemented)")
        );

        // ── SecAccessControl ────────────────────────────────────
        register_shims!(table,
            ("_SecAccessControlCreateWithFlags", shim_sec_return_null3 as u64, "SecAccessControlCreateWithFlags (stub → NULL)")
        );

        // ── Randomness ──────────────────────────────────────────
        register_shims!(table,
            ("_SecRandomCopyBytes", shim_sec_random_copy_bytes as u64, "SecRandomCopyBytes (stub → fills with random)")
        );

        // ── PKCS12 / import-export ──────────────────────────────
        register_shims!(table,
            ("_SecPKCS12Import", shim_sec_return_unimplemented as u64, "SecPKCS12Import (stub → errSecUnimplemented)")
        );

        // ── Keychain refs (legacy) ──────────────────────────────
        register_shims!(table,
            ("_SecKeychainItemCopyContent", shim_sec_return_unimplemented as u64, "SecKeychainItemCopyContent (stub → errSecUnimplemented)"),
            ("_SecKeychainFindGenericPassword", shim_sec_return_item_not_found as u64, "SecKeychainFindGenericPassword (stub → errSecItemNotFound)"),
            ("_SecKeychainFindInternetPassword", shim_sec_return_item_not_found as u64, "SecKeychainFindInternetPassword (stub → errSecItemNotFound)"),
            ("_SecKeychainAddGenericPassword", shim_sec_return_unimplemented as u64, "SecKeychainAddGenericPassword (stub → errSecUnimplemented)")
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
            ("_AuthorizationCreate",                   shim_auth_denied as u64,        "AuthorizationCreate (stub → errAuthorizationDenied)"),
            ("_AuthorizationFree",                     shim_auth_free as u64,           "AuthorizationFree (stub → void)"),
            ("_AuthorizationCopyRights",               shim_auth_denied as u64,        "AuthorizationCopyRights (stub → errAuthorizationDenied)"),
            ("_AuthorizationCopyRightsAsync",          shim_auth_copy_rights_async as u64, "AuthorizationCopyRightsAsync (stub → void/denied)"),
            ("_AuthorizationMakeExternalForm",         shim_auth_denied as u64,        "AuthorizationMakeExternalForm (stub → errAuthorizationDenied)"),
            ("_AuthorizationCreateFromExternalForm",   shim_auth_denied as u64,        "AuthorizationCreateFromExternalForm (stub → errAuthorizationDenied)"),
            ("_AuthorizationCopyInfo",                 shim_auth_denied as u64,        "AuthorizationCopyInfo (stub → errAuthorizationDenied)"),
            ("_AuthorizationExecuteWithPrivileges",    shim_auth_denied as u64,        "AuthorizationExecuteWithPrivileges (stub → errAuthorizationDenied)"),
            ("_AuthorizationCancelRight",              shim_auth_cancelled as u64,     "AuthorizationCancelRight (stub → errAuthorizationCanceled)"),
            ("_AuthorizationFreeItemSet",              shim_auth_free_item_set as u64, "AuthorizationFreeItemSet (stub → void)"),
            ("_AuthorizationRightSet",                 shim_auth_denied as u64,        "AuthorizationRightSet (stub → errAuthorizationDenied)"),
            ("_AuthorizationRightGet",                 shim_auth_denied as u64,        "AuthorizationRightGet (stub → errAuthorizationDenied)"),
            ("_AuthorizationRightRemove",              shim_auth_denied as u64,        "AuthorizationRightRemove (stub → errAuthorizationDenied)"),
            ("_AuthorizationPluginCreate",             shim_auth_denied as u64,        "AuthorizationPluginCreate (stub → errAuthorizationDenied)"),
            // Private Authorization SPI used by /usr/bin/security
            ("_AuthorizationCopyPreloginUserDatabase", shim_auth_denied as u64,        "AuthorizationCopyPreloginUserDatabase (stub → errAuthorizationDenied)"),
            ("_AuthorizationHandlePreloginOverride",   shim_auth_denied as u64,        "AuthorizationHandlePreloginOverride (stub → errAuthorizationDenied)"),
        );

        // ── Network.framework (nw_* symbols) ───────────────────
        // Network.framework is not available in our stub environment.
        // Return NULL / no-op for all nw_* calls.
        register_shims!(table,
            ("_nw_connection_create",              shim_sec_return_null as u64, "nw_connection_create (stub → NULL)"),
            ("_nw_connection_start",               shim_void1_stub as u64,      "nw_connection_start (stub → void)"),
            ("_nw_connection_send",                shim_void4_stub as u64,      "nw_connection_send (stub → void)"),
            ("_nw_connection_receive",             shim_void4_stub as u64,      "nw_connection_receive (stub → void)"),
            ("_nw_connection_cancel",              shim_void1_stub as u64,      "nw_connection_cancel (stub → void)"),
            ("_nw_connection_set_state_changed_handler", shim_void2_stub as u64, "nw_connection_set_state_changed_handler (stub → void)"),
            ("_nw_connection_set_queue",           shim_void2_stub as u64,      "nw_connection_set_queue (stub → void)"),
            ("_nw_endpoint_create_host",           shim_sec_return_null as u64, "nw_endpoint_create_host (stub → NULL)"),
            ("_nw_parameters_create_secure_tcp",   shim_sec_return_null as u64, "nw_parameters_create_secure_tcp (stub → NULL)"),
            ("_nw_parameters_create_secure_udp",   shim_sec_return_null as u64, "nw_parameters_create_secure_udp (stub → NULL)"),
            ("_nw_parameters_create",              shim_sec_return_null as u64, "nw_parameters_create (stub → NULL)"),
            ("_nw_listener_create",                shim_sec_return_null as u64, "nw_listener_create (stub → NULL)"),
            ("_nw_listener_start",                 shim_void1_stub as u64,      "nw_listener_start (stub → void)"),
            ("_nw_listener_cancel",                shim_void1_stub as u64,      "nw_listener_cancel (stub → void)"),
            ("_nw_listener_set_state_changed_handler", shim_void2_stub as u64, "nw_listener_set_state_changed_handler (stub → void)"),
            ("_nw_listener_set_new_connection_handler", shim_void2_stub as u64, "nw_listener_set_new_connection_handler (stub → void)"),
            ("_nw_listener_set_queue",             shim_void2_stub as u64,      "nw_listener_set_queue (stub → void)"),
            ("_nw_path_monitor_create",            shim_sec_return_null as u64, "nw_path_monitor_create (stub → NULL)"),
            ("_nw_path_monitor_start",             shim_void1_stub as u64,      "nw_path_monitor_start (stub → void)"),
            ("_nw_path_monitor_set_queue",         shim_void2_stub as u64,      "nw_path_monitor_set_queue (stub → void)"),
            ("_nw_path_monitor_set_update_handler",shim_void2_stub as u64,      "nw_path_monitor_set_update_handler (stub → void)"),
            ("_nw_path_monitor_cancel",            shim_void1_stub as u64,      "nw_path_monitor_cancel (stub → void)"),
            // Additional nw_* symbols imported by /usr/bin/security
            ("__nw_parameters_configure_protocol_default_configuration", shim_void1_stub as u64, "nw_parameters_configure_protocol_default_configuration (stub → void)"),
            ("_nw_connection_copy_endpoint",       shim_sec_return_null as u64, "nw_connection_copy_endpoint (stub → NULL)"),
            ("_nw_endpoint_get_hostname",          shim_sec_return_null as u64, "nw_endpoint_get_hostname (stub → NULL)"),
            ("_nw_endpoint_get_port",              shim_sec_return_null as u64, "nw_endpoint_get_port (stub → NULL)"),
            ("_nw_error_get_error_code",           shim_sec_return_null as u64, "nw_error_get_error_code (stub → 0)"),
            ("_nw_parameters_set_indefinite",      shim_void2_stub as u64,      "nw_parameters_set_indefinite (stub → void)"),
            ("_nw_tls_copy_sec_protocol_options",  shim_sec_return_null as u64, "nw_tls_copy_sec_protocol_options (stub → NULL)"),
        );

        // ── CSSM (deprecated, legacy CDSA crypto) ───────────────
        register_shims!(table,
            ("_CSSM_Init",        shim_auth_denied as u64, "CSSM_Init (stub → error)"),
            ("_CSSM_ModuleLoad",  shim_auth_denied as u64, "CSSM_ModuleLoad (stub → error)"),
            ("_CSSM_ModuleUnload",shim_auth_denied as u64, "CSSM_ModuleUnload (stub → error)"),
            ("_CSSM_Terminate",   shim_auth_denied as u64, "CSSM_Terminate (stub → error)"),
            // CSSM string helpers (lowercase prefix, need explicit stubs)
            ("_cssmErrorString",   shim_sec_return_null as u64, "cssmErrorString (stub → NULL)"),
            ("_cssmPerror",        shim_void1_stub as u64,      "cssmPerror (stub → void)"),
        );

        // ── SecLogObj / diagnostics ──────────────────────────────
        register_shims!(table,
            ("_secLogObjForScope", shim_sec_return_null as u64, "secLogObjForScope (stub → NULL)"),
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

// ── In-memory keychain store ────────────────────────────────────

/// A keychain item stored in memory.
#[derive(Clone)]
struct KeychainItem {
    class: u64,      // sentinel value (KSEC_CLASS_GENERIC_PW etc.)
    service: String,  // kSecAttrService value (extracted string)
    account: String,  // kSecAttrAccount value (extracted string)
    data: Vec<u8>,    // kSecValueData payload
}

static KEYCHAIN: Mutex<Option<HashMap<(String, String), KeychainItem>>> = Mutex::new(None);

fn keychain_store() -> std::sync::MutexGuard<'static, Option<HashMap<(String, String), KeychainItem>>> {
    KEYCHAIN.lock().unwrap_or_else(|e| e.into_inner())
}

fn ensure_store(guard: &mut Option<HashMap<(String, String), KeychainItem>>) -> &mut HashMap<(String, String), KeychainItem> {
    guard.get_or_insert_with(HashMap::new)
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
/// NSDictionary layout: [isa:8][count:8][keys_ptr:8][vals_ptr:8]
fn dict_value_for_sentinel(dict: u64, sentinel: u64) -> u64 {
    if dict < 0x1000 { return 0; }
    let count = raw_read_u64(dict, 8) as usize;
    let keys_ptr = raw_read_u64(dict, 16);
    let vals_ptr = raw_read_u64(dict, 24);
    if keys_ptr == 0 || vals_ptr == 0 || count == 0 { return 0; }
    for i in 0..count {
        let k = unsafe { *((keys_ptr as usize + i * 8) as *const u64) };
        if k == sentinel {
            return unsafe { *((vals_ptr as usize + i * 8) as *const u64) };
        }
    }
    0
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

// ── Shim functions ──────────────────────────────────────────────

/// SecItemAdd(attributes, result) → OSStatus
pub extern "C" fn shim_sec_item_add(attrs: u64, result: *mut u64) -> i32 {
    let (class, service, account, data) = parse_query(attrs);
    if class == 0 {
        return ERR_SEC_UNIMPLEMENTED;
    }
    let item = KeychainItem {
        class,
        service: service.clone(),
        account: account.clone(),
        data: data.unwrap_or_default(),
    };
    let key = (service, account);
    let mut guard = keychain_store();
    let store = ensure_store(&mut guard);
    if store.contains_key(&key) {
        return ERR_SEC_DUPLICATE_ITEM;
    }
    store.insert(key, item);
    if !result.is_null() {
        unsafe { *result = 0; } // no result object returned on add
    }
    ERR_SEC_SUCCESS
}

/// SecItemCopyMatching(query, result) → OSStatus
pub extern "C" fn shim_sec_item_copy_matching(query: u64, result: *mut u64) -> i32 {
    let (_class, service, account, _data) = parse_query(query);
    let key = (service, account);
    let guard = keychain_store();
    if let Some(store) = guard.as_ref() {
        if let Some(item) = store.get(&key) {
            let return_data = dict_value_for_sentinel(query, KSEC_RETURN_DATA);
            if return_data != 0 && !result.is_null() {
                // Return an NSData object containing the stored data.
                // We allocate a minimal NSData-like object: [isa:8][bytes_ptr:8][length:8]
                let data_obj = unsafe { libc::calloc(1, 24) as u64 };
                if data_obj != 0 && !item.data.is_empty() {
                    let buf = unsafe { libc::malloc(item.data.len()) as *mut u8 };
                    if !buf.is_null() {
                        unsafe {
                            std::ptr::copy_nonoverlapping(item.data.as_ptr(), buf, item.data.len());
                            *((data_obj as usize + 8) as *mut u64) = buf as u64;
                            *((data_obj as usize + 16) as *mut u64) = item.data.len() as u64;
                        }
                    }
                }
                unsafe { *result = data_obj; }
                return ERR_SEC_SUCCESS;
            }
            if !result.is_null() {
                unsafe { *result = 0; }
            }
            return ERR_SEC_SUCCESS;
        }
    }
    if !result.is_null() {
        unsafe { *result = 0; }
    }
    ERR_SEC_ITEM_NOT_FOUND
}

/// SecItemUpdate(query, attrs) → OSStatus
pub extern "C" fn shim_sec_item_update(query: u64, attrs: u64) -> i32 {
    let (_class, service, account, _data) = parse_query(query);
    let key = (service, account);
    let (_class2, _service2, _account2, new_data) = parse_query(attrs);
    let mut guard = keychain_store();
    let store = ensure_store(&mut guard);
    if let Some(item) = store.get_mut(&key) {
        if let Some(d) = new_data {
            item.data = d;
        }
        return ERR_SEC_SUCCESS;
    }
    ERR_SEC_ITEM_NOT_FOUND
}

/// SecItemDelete(query) → OSStatus
pub extern "C" fn shim_sec_item_delete(query: u64) -> i32 {
    let (_class, service, account, _data) = parse_query(query);
    let key = (service, account);
    let mut guard = keychain_store();
    let store = ensure_store(&mut guard);
    if store.remove(&key).is_some() {
        return ERR_SEC_SUCCESS;
    }
    ERR_SEC_ITEM_NOT_FOUND
}

/// SecCertificateCreateWithData(allocator, data) → SecCertificateRef
pub extern "C" fn shim_sec_cert_create(_alloc: u64, _data: u64) -> u64 {
    0 // NULL — can't create real certs
}

/// SecCertificateCopyCommonName(cert, &commonName) → OSStatus
pub extern "C" fn shim_sec_copy_common_name(_cert: u64, _name: *mut u64) -> i32 {
    ERR_SEC_UNIMPLEMENTED
}

/// Generic return-NULL for 1-arg functions
pub extern "C" fn shim_sec_return_null(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_sec_return_null3(_a: u64, _b: u64, _c: *mut u64) -> u64 { 0 }
pub extern "C" fn shim_sec_return_false(_a: u64, _b: u64, _c: u64) -> i32 { 0 }
pub extern "C" fn shim_sec_return_zero(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_sec_return_unimplemented(_a: u64) -> i32 { ERR_SEC_UNIMPLEMENTED }
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
pub extern "C" fn shim_sec_key_create_random(_params: u64, error: *mut u64) -> u64 {
    if !error.is_null() {
        // TODO: create a CFError indicating unsupported
        unsafe { *error = 0; }
    }
    0 // NULL — can't generate keys
}

/// SecTrustCreateWithCertificates(certs, policies, &trust) → OSStatus
pub extern "C" fn shim_sec_trust_create(_certs: u64, _policies: u64, _trust: *mut u64) -> i32 {
    ERR_SEC_UNIMPLEMENTED
}

/// SecTrustEvaluateWithError(trust, &error) → bool
pub extern "C" fn shim_sec_trust_evaluate_with_error(_trust: u64, _error: *mut u64) -> i32 {
    0 // false — trust evaluation failed
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
/// Uses libc arc4random_buf or getrandom to fill with real random bytes.
pub extern "C" fn shim_sec_random_copy_bytes(_rng: u64, count: usize, bytes: *mut u8) -> i32 {
    if bytes.is_null() || count == 0 {
        return ERR_SEC_SUCCESS;
    }
    unsafe {
        // Use /dev/urandom as fallback
        let slice = std::slice::from_raw_parts_mut(bytes, count);
        #[cfg(target_os = "macos")]
        {
            libc::arc4random_buf(slice.as_mut_ptr() as *mut libc::c_void, count);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let fd = libc::open(b"/dev/urandom\0".as_ptr() as *const i8, libc::O_RDONLY);
            if fd >= 0 {
                libc::read(fd, slice.as_mut_ptr() as *mut libc::c_void, count);
                libc::close(fd);
            } else {
                // Last resort: zero-fill
                slice.fill(0);
                return ERR_SEC_UNIMPLEMENTED;
            }
        }
    }
    ERR_SEC_SUCCESS
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
        let mut guard = keychain_store();
        if let Some(store) = guard.as_mut() {
            store.clear();
        }
    }

    #[test]
    fn keychain_add_and_query() {
        clear_keychain();
        let svc = make_fake_cfstring("com.test.myapp");
        let acct = make_fake_cfstring("user@example.com");
        let data = make_fake_nsdata(b"s3cr3t");

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
        clear_keychain();
    }

    #[test]
    fn keychain_duplicate_add_fails() {
        clear_keychain();
        let svc = make_fake_cfstring("dup.test");
        let acct = make_fake_cfstring("alice");
        let data = make_fake_nsdata(b"pw1");

        let dict = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_VALUE_DATA, data),
        ]);

        assert_eq!(shim_sec_item_add(dict, std::ptr::null_mut()), ERR_SEC_SUCCESS);
        assert_eq!(shim_sec_item_add(dict, std::ptr::null_mut()), ERR_SEC_DUPLICATE_ITEM);
        clear_keychain();
    }

    #[test]
    fn keychain_update() {
        clear_keychain();
        let svc = make_fake_cfstring("upd.test");
        let acct = make_fake_cfstring("bob");
        let data1 = make_fake_nsdata(b"old");
        let data2 = make_fake_nsdata(b"new_password");

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
        clear_keychain();
    }

    #[test]
    fn keychain_delete() {
        clear_keychain();
        let svc = make_fake_cfstring("del.test");
        let acct = make_fake_cfstring("carol");
        let data = make_fake_nsdata(b"delete_me");

        let add_dict = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
            (KSEC_VALUE_DATA, data),
        ]);
        assert_eq!(shim_sec_item_add(add_dict, std::ptr::null_mut()), ERR_SEC_SUCCESS);

        let del_query = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        assert_eq!(shim_sec_item_delete(del_query), ERR_SEC_SUCCESS);
        // Delete again → not found
        assert_eq!(shim_sec_item_delete(del_query), ERR_SEC_ITEM_NOT_FOUND);
        clear_keychain();
    }

    #[test]
    fn keychain_query_nonexistent() {
        clear_keychain();
        let svc = make_fake_cfstring("no.such.service");
        let acct = make_fake_cfstring("nobody");
        let query = make_fake_dict(&[
            (KSEC_CLASS, KSEC_CLASS_GENERIC_PW),
            (KSEC_ATTR_SERVICE, svc),
            (KSEC_ATTR_ACCOUNT, acct),
        ]);
        let mut result: u64 = 42;
        assert_eq!(shim_sec_item_copy_matching(query, &mut result), ERR_SEC_ITEM_NOT_FOUND);
        assert_eq!(result, 0);
    }
}
