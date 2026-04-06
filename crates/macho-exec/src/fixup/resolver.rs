//! StubResolver: maps known-safe symbols to host process addresses.

use platform_core::symbols::HostSymbolResolver;
use std::sync::{Arc, Once};

/// Trait for resolving symbol names to in-process addresses.
pub trait SymbolResolver {
    fn resolve(&self, symbol_name: &str) -> Option<u64>;
}

/// Resolves symbol stubs to real in-process function addresses.
///
/// Only resolves symbols on a curated safe-list to prevent dangerous
/// side effects during lab execution.
pub struct StubResolver {
    host: Arc<dyn HostSymbolResolver>,
    handle: *mut (),
    /// Separate handle for Apple-ABI libc++ (loaded with RTLD_LOCAL).
    /// C++ symbols are resolved from this first to match macOS ABI.
    #[cfg(target_os = "linux")]
    cxx_handle: Option<*mut ()>,
}

// StubResolver's handle is an opaque RTLD_DEFAULT / dlopen handle.
// It is read-only and valid for the process lifetime.
unsafe impl Send for StubResolver {}
unsafe impl Sync for StubResolver {}

struct PreloadState {
    once: Once,
}

impl PreloadState {
    const fn new() -> Self {
        Self { once: Once::new() }
    }

    fn preload(&self, host: &dyn HostSymbolResolver, libraries: &[&str]) {
        self.once.call_once(|| {
            for lib in libraries {
                if let Some(_handle) = host.open_library(lib) {
                    log::debug!("StubResolver: preloaded {lib}");
                } else {
                    log::trace!("StubResolver: could not preload {lib}");
                }
            }
        });
    }
}

static PRELOAD_STATE: PreloadState = PreloadState::new();

impl StubResolver {
    /// Libraries to preload so their symbols are available via RTLD_DEFAULT.
    /// Without this, dlsym can't find symbols from frameworks/libraries
    /// that the host process doesn't directly link.
    const PRELOAD_LIBRARIES: &[&str] = &[
        "/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation",
        "/System/Library/Frameworks/Foundation.framework/Foundation",
        "/System/Library/Frameworks/Security.framework/Security",
        "/System/Library/Frameworks/CoreServices.framework/CoreServices",
        "/System/Library/Frameworks/IOKit.framework/IOKit",
        "/System/Library/Frameworks/SystemConfiguration.framework/SystemConfiguration",
        "/System/Library/Frameworks/GSS.framework/GSS",
        "/System/Library/Frameworks/Kerberos.framework/Kerberos",
        "libxo.dylib",
        "libedit.dylib",
        "/usr/lib/libresolv.9.dylib",
        "/usr/lib/libz.1.dylib",
        "/usr/lib/libbz2.1.0.dylib",
        "/usr/lib/liblzma.5.dylib",
        "/usr/lib/libarchive.2.dylib",
        "/usr/lib/libcurl.4.dylib",
        "libncurses.dylib",
        "libpanel.dylib",
        "/usr/lib/libutil.dylib",
        "libxcselect.dylib",
        "/usr/lib/swift/libswiftCore.dylib",
        "/usr/lib/swift/libswiftCoreFoundation.dylib",
        "/usr/lib/swift/libswiftDispatch.dylib",
        "/usr/lib/swift/libswiftIOKit.dylib",
        "/usr/lib/swift/libswiftObjectiveC.dylib",
        "/usr/lib/swift/libswiftXPC.dylib",
        "/usr/lib/swift/libswift_Builtin_float.dylib",
        "/usr/lib/swift/libswift_Concurrency.dylib",
        "/usr/lib/swift/libswift_StringProcessing.dylib",
        "/System/Library/Frameworks/CryptoKit.framework/CryptoKit",
        "/System/Library/PrivateFrameworks/AuthKit.framework/AuthKit",
        "/usr/lib/libxar.1.dylib",
        // SceneKit data symbols (scntool, compileSceneKitShaders, TextureConverter)
        "/System/Library/Frameworks/SceneKit.framework/SceneKit",
        // CoreGraphics + ImageIO for CG*/CGImage* symbols
        "/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics",
        "/System/Library/Frameworks/ImageIO.framework/ImageIO",
        // ICU (International Components for Unicode) — uenum_*, ucol_*, u_* etc.
        "/usr/lib/libicucore.A.dylib",
        // Carbon for DeRez/Rez Resource Manager APIs
        "/System/Library/Frameworks/Carbon.framework/Carbon",
        // CoreText for CTFont/CTParagraphStyle symbols
        "/System/Library/Frameworks/CoreText.framework/CoreText",
        // DiskArbitration for DASession/DADisk symbols
        "/System/Library/Frameworks/DiskArbitration.framework/DiskArbitration",
        // QuartzCore (CoreAnimation) for CA* symbols
        "/System/Library/Frameworks/QuartzCore.framework/QuartzCore",
        // AudioToolbox for AudioSession/AudioUnit symbols
        "/System/Library/Frameworks/AudioToolbox.framework/AudioToolbox",
        // UserNotifications for UNUserNotificationCenter symbols
        "/System/Library/Frameworks/UserNotifications.framework/UserNotifications",
        // libxml2 for xml* symbols
        "/usr/lib/libxml2.2.dylib",
        // ServiceManagement for SMJobBless and related
        "/System/Library/Frameworks/ServiceManagement.framework/ServiceManagement",
        // SwiftUI — protocol witnesses and conformances used by Xcode tools
        "/System/Library/Frameworks/SwiftUI.framework/SwiftUI",
        // Swift regex engine and string processing modules
        "/usr/lib/swift/libswift_RegexParser.dylib",
        "/usr/lib/swift/libswift_StringProcessing.dylib",
    ];

    /// Linux shared libraries to preload so their symbols are available via
    /// RTLD_DEFAULT.  These are the Linux equivalents of the macOS framework
    /// libraries listed in PRELOAD_LIBRARIES.
    #[cfg(target_os = "linux")]
    const LINUX_PRELOAD_LIBRARIES: &[&str] = &[
        "libbz2.so.1.0",
        "liblzma.so.5",
        "libz.so.1",
        "libarchive.so.13",
        "libssl.so.3",
        "libcrypto.so.3",
        "/usr/local/lib/libressl_compat.so",
        "libacl.so.1",
        "libc++.so.1",
        "libc++abi.so.1",
    ];

    /// Path to a custom libc++ built with Apple's `_LIBCPP_ABI_ALTERNATE_STRING_LAYOUT`.
    /// This library is dlopen'd with RTLD_LOCAL so it doesn't interfere with
    /// the host process's libc++. C++ symbols are resolved from this library
    /// first to ensure ABI compatibility with macOS Mach-O binaries.
    #[cfg(target_os = "linux")]
    const APPLE_ABI_LIBCXX_PATH: &str = "/opt/apple-libcxx/libc++.so.1.0";

    pub fn new(host: Arc<dyn HostSymbolResolver>) -> Self {
        // Preload host libraries once per process so RTLD_DEFAULT can see them
        // without paying repeated dlopen/dyld setup cost on every loader run.
        #[cfg(target_os = "macos")]
        PRELOAD_STATE.preload(host.as_ref(), Self::PRELOAD_LIBRARIES);
        #[cfg(target_os = "linux")]
        PRELOAD_STATE.preload(host.as_ref(), Self::LINUX_PRELOAD_LIBRARIES);
        let handle = host.default_handle();

        // On Linux, try to open the Apple-ABI libc++ for C++ symbol resolution.
        // Must use RTLD_LOCAL to avoid replacing the system libc++ symbols
        // that the host process depends on.
        #[cfg(target_os = "linux")]
        let cxx_handle = {
            let path = std::ffi::CString::new(Self::APPLE_ABI_LIBCXX_PATH).ok();
            let h = path.and_then(|p| {
                let h = unsafe { libc::dlopen(p.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
                if h.is_null() { None } else { Some(h as *mut ()) }
            });
            if h.is_some() {
                log::info!("StubResolver: loaded Apple-ABI libc++ from {} (RTLD_LOCAL)", Self::APPLE_ABI_LIBCXX_PATH);
            } else {
                log::debug!("StubResolver: Apple-ABI libc++ not found at {}, using system libc++", Self::APPLE_ABI_LIBCXX_PATH);
            }
            h
        };

        Self {
            host,
            handle,
            #[cfg(target_os = "linux")]
            cxx_handle,
        }
    }

    /// Check if a symbol is a std::basic_string method that needs the Apple
    /// alternate string layout. Only these symbols are redirected to the
    /// Apple-ABI libc++ to avoid RTTI conflicts.
    #[cfg(target_os = "linux")]
    fn is_apple_cxx_symbol(name: &str) -> bool {
        // std::__1::basic_string<char, ...> member functions and related
        // Mangled prefix: _ZNSt3__112basic_stringIc  or _ZNKSt3__112basic_stringIc
        if name.contains("12basic_stringIc") {
            return true;
        }
        // std::__1::operator+(const char*, const basic_string<char>&)
        // Mangled: _ZNSt3__1plIc
        if name.contains("St3__1plIc") {
            return true;
        }
        false
    }

    pub fn resolve(&self, name: &str) -> Option<u64> {
        let c_name = name.strip_prefix('_').unwrap_or(name);
        // Strip Darwin variant suffixes ($DARWIN_EXTSN, $UNIX2003, etc.)
        // On Linux, all functions are POSIX-conformant by default.
        let c_name = c_name.split('$').next().unwrap_or(c_name);
        if !Self::is_safe_symbol(c_name) {
            log::debug!("StubResolver: skipping unsafe symbol: {name}");
            runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::SymbolBlocked {
                name: name.into(),
            });
            return None;
        }

        // On Linux, try the Apple-ABI libc++ first for std::string symbols.
        // Only redirect basic_string-related symbols to avoid RTTI/type_info
        // conflicts that cause std::bad_cast when two libc++ instances coexist.
        #[cfg(target_os = "linux")]
        if let Some(cxx_h) = self.cxx_handle {
            if Self::is_apple_cxx_symbol(c_name) {
                if let Some(ptr) = self.host.resolve_in(cxx_h, c_name) {
                    return Some(ptr as u64);
                }
            }
        }

        let addr = self.host.resolve_in(self.handle, c_name);
        match addr {
            Some(ptr) => Some(ptr as u64),
            None => {
                log::debug!("StubResolver: host returned null for {c_name}");
                runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::SymbolMissing {
                    name: name.into(),
                });
                None
            }
        }
    }

    fn is_safe_symbol(name: &str) -> bool {
        // C++ ABI symbols (libc++, libstdc++) — mangled names start with _Z
        if name.starts_with("_Z") || name.starts_with("__Z") {
            return true;
        }
        // C++ guard / atexit / RTTI
        if name.starts_with("__cxa_") || name == "__dynamic_cast" {
            return true;
        }
        // libcompression / libz
        if name.starts_with("compress") || name.starts_with("uncompress")
            || name.starts_with("deflate") || name.starts_with("inflate")
            || name.starts_with("crc32") || name.starts_with("adler32")
        {
            return true;
        }
        // ObjC class/metaclass/ehtype data symbols — resolved via dlsym like functions
        if name.starts_with("OBJC_CLASS_$_")
            || name.starts_with("OBJC_METACLASS_$_")
            || name.starts_with("OBJC_EHTYPE_")
        {
            return true;
        }
        // Carbon HFS+ / File System Services (CoreServices.CarbonCore)
        // e.g. FSPathMakeRef, FSOpenResourceFile, FSReadFork, FSGetForkSize, …
        if name.starts_with("FS") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // Uniform Type Identifiers — constants (kUTTagClassFilenameExtension, …)
        if name.starts_with("kUT") {
            return true;
        }
        // UTType functions (UTTypeCreatePreferredIdentifierForTag, …)
        if name.starts_with("UT") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // Open Scripting Architecture (sdef, osascript, …)
        if name.starts_with("OSA") {
            return true;
        }
        // CoreEndian — byte-swapping for resource-fork data
        if name.starts_with("CoreEndian") {
            return true;
        }
        // SceneKit constants not covered by the SC* check (kC3DIO*, kSCN*)
        if name.starts_with("kC3DIO") || name.starts_with("kSCN") {
            return true;
        }
        // GeoKit / GameplayKit / GameKit / CloudKit / ARKit / RealityKit prefixes
        if (name.starts_with("GK") || name.starts_with("CK") || name.starts_with("AR")
            || name.starts_with("RK") || name.starts_with("MP"))
            && name.len() > 2
            && name.as_bytes()[2].is_ascii_uppercase()
        {
            return true;
        }
        // CoreGraphics / ImageIO / Quartz symbols (CG*)
        if name.starts_with("CG") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // ICU (International Components for Unicode) — uenum_*, ucol_*, u_*, etc.
        if name.starts_with("uenum_") || name.starts_with("ucol_") || name.starts_with("uldn_")
            || name.starts_with("uloc_") || name.starts_with("u_") || name.starts_with("ucnv_")
            || name.starts_with("ubrk_") || name.starts_with("utext_")
            || name.starts_with("unorm2_") || name.starts_with("udat_")
            || name.starts_with("unum_") || name.starts_with("ucal_")
        {
            return true;
        }
        // Apple System Information private framework (ASI_*)
        if name.starts_with("ASI_") {
            return true;
        }
        // MobileGestalt / private device info
        if name.starts_with("MGCopyAnswer") || name.starts_with("MGGetBool") {
            return true;
        }
        // Rosetta / OAH translation (oah_*)
        if name.starts_with("oah_") || name.starts_with("OAH") {
            return true;
        }
        // Power management private (pm_*)
        if name.starts_with("pm_") {
            return true;
        }
        // BackBoardServices (BKS*), FrontBoardServices (FBS*)
        if name.starts_with("BKS") || name.starts_with("FBS") {
            return true;
        }
        // CoreEntitlements (CE*)
        if name.starts_with("CE") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // Analytics / diagnostics
        if name.starts_with("AnalyticsSend") {
            return true;
        }
        // MobileInstallation / Provisioning profiles (MIS*, CP*)
        if name.starts_with("MIS") || name.starts_with("CPInstalled") {
            return true;
        }
        // CoreFoundation constant data symbols and function prefixes
        if name.starts_with("__CFConstant") || name.starts_with("kCF") {
            return true;
        }
        // CF API functions — broad prefix for CoreFoundation (public and private)
        if (name.starts_with("CF") || name.starts_with("_CF"))
            && name.len() > 3
            && name.as_bytes()[if name.starts_with("_CF") { 3 } else { 2 }].is_ascii_uppercase()
        {
            return true;
        }
        // LTO / ThinLTO API (libLTO.dylib)
        if name.starts_with("lto_") || name.starts_with("thinlto_") {
            return true;
        }
        // libarchive
        if name.starts_with("archive_") {
            return true;
        }
        // XAR archives (libxar)
        if name.starts_with("xar_") {
            return true;
        }
        // DiskImages framework
        if name.starts_with("DI") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // XPC framework
        if name.starts_with("xpc_") {
            return true;
        }
        // libSystem _simple (internal)
        if name.starts_with("_simple_") {
            return true;
        }
        // os_log / logging
        if name.starts_with("os_log") || name.starts_with("_os_log")
            || name.starts_with("os_signpost") || name.starts_with("_os_signpost")
        {
            return true;
        }
        // Swift force-load symbols
        if name.starts_with("_swift_FORCE_LOAD") {
            return true;
        }
        // Swift mangled symbols and runtime entry points
        if name.starts_with("$s") || name.starts_with("$S")
            || name.starts_with("_$s") || name.starts_with("_$S")
            || name.starts_with("swift_")
            || name.starts_with("_swift")
        {
            return true;
        }
        // sqlite3
        if name.starts_with("sqlite3_") {
            return true;
        }
        // FSEvents
        if name.starts_with("FSEvent") {
            return true;
        }
        // Security framework constants (kSec*)
        if name.starts_with("kSec") {
            return true;
        }
        // CommonCrypto
        if name.starts_with("CC_SHA") || name.starts_with("CC_MD5")
            || name.starts_with("CCCryptor") || name.starts_with("CCKey")
            || name.starts_with("CCHmac") || name.starts_with("CCPbkdf")
            || name.starts_with("CCDigest") || name.starts_with("CC_")
        {
            return true;
        }
        // Mach clock APIs (libSystem)
        if name.starts_with("clock_get_time") || name.starts_with("host_get_clock_service")
            || name.starts_with("mach_port_deallocate") || name.starts_with("mach_host_self")
        {
            return true;
        }
        // DebugSymbols.framework private APIs (symbols, filtercalltree tool)
        if name.starts_with("DBGCopy") || name.starts_with("DBGCreate")
            || name.starts_with("aot_address_in") || name.starts_with("aot_get_")
        {
            return true;
        }
        // CFPreferences
        if name.starts_with("CFPreferences") || name.starts_with("_CFPreferences")
            || name.starts_with("_CFPref") || name.starts_with("_CFX")
        {
            return true;
        }
        // libcodesign
        if name.starts_with("libcd_") {
            return true;
        }
        // malloc_type (typed allocations)
        if name.starts_with("malloc_type_") {
            return true;
        }
        // libxo output
        if name.starts_with("xo_") {
            return true;
        }
        // quarantine
        if name.starts_with("_qtn_") {
            return true;
        }
        // ACL
        if name.starts_with("acl_") {
            return true;
        }
        // lzma / liblzma
        if name.starts_with("lzma_") {
            return true;
        }
        // BZ2 / bzip2
        if name.starts_with("BZ2_") {
            return true;
        }
        // OpenSSL / LibreSSL — macOS ships LibreSSL whose symbols are mostly
        // ABI-compatible with OpenSSL 3.  Prefixes cover both the public API
        // and internal helpers used by the macOS `openssl` CLI.
        if name.starts_with("SSL_") || name.starts_with("TLS_")
            || name.starts_with("TLSv") || name.starts_with("DTLS_")
            || name.starts_with("EVP_") || name.starts_with("PEM_")
            || name.starts_with("BIO_") || name.starts_with("BN_")
            || name.starts_with("RSA_") || name.starts_with("DSA_")
            || name.starts_with("DH_") || name.starts_with("EC_")
            || name.starts_with("ECDSA_") || name.starts_with("ECDH_")
            || name.starts_with("X509") || name.starts_with("ASN1_")
            || name.starts_with("OBJ_") || name.starts_with("OCSP_")
            || name.starts_with("PKCS") || name.starts_with("CMS_")
            || name.starts_with("HMAC") || name.starts_with("RAND_")
            || name.starts_with("CRYPTO_") || name.starts_with("ERR_")
            || name.starts_with("OPENSSL_") || name.starts_with("CONF_")
            || name.starts_with("NCONF_") || name.starts_with("ENGINE_")
            // Cipher primitives
            || name.starts_with("AES_") || name.starts_with("BF_")
            || name.starts_with("CAST_") || name.starts_with("Camellia_")
            || name.starts_with("DES_") || name.starts_with("RC2_")
            || name.starts_with("RC4") // RC4, RC4_options, RC4_set_key
            // Hash primitives
            || name.starts_with("SHA1") || name.starts_with("SHA256")
            || name.starts_with("SHA512") || name.starts_with("WHIRLPOOL")
            // Buffer / UI / TXT_DB / SMIME
            || name.starts_with("BUF_MEM_") || name.starts_with("BUF_strlcpy")
            || name.starts_with("UI_") || name.starts_with("TXT_DB_")
            || name.starts_with("SMIME_")
            // Param print/dup helpers
            || name.starts_with("DHparams_") || name.starts_with("DSAparams_")
            || name.starts_with("ECPKParameters_") || name.starts_with("PBEPARAM_")
            // X509 type structs
            || name.starts_with("GENERAL_NAME")
            || name.starts_with("AUTHORITY_INFO")
            || name.starts_with("ACCESS_DESCRIPTION")
            || name.starts_with("BASIC_CONSTRAINTS")
            || name.starts_with("DIST_POINT")
            || name.starts_with("ISSUING_DIST_POINT")
            || name.starts_with("POLICYINFO")
            || name.starts_with("POLICY_MAPPING")
            || name.starts_with("NAME_CONSTRAINTS")
            || name.starts_with("NETSCAPE_")
            // Encoding helpers
            || name.starts_with("i2d_") || name.starts_with("d2i_")
            || name.starts_with("i2a_") || name.starts_with("a2i_")
            || name.starts_with("i2t_") || name.starts_with("i2v_")
            || name.starts_with("i2b_") || name.starts_with("b2i_")
            || name.starts_with("s2i_")
            || name.starts_with("sk_") || name.starts_with("lh_")
            || name.starts_with("CTLOG_")
            || name.starts_with("CT_") || name.starts_with("SCT_")
            || name.starts_with("TS_")
        {
            return true;
        }
        // Standalone OpenSSL symbols (not prefix-matched above)
        if matches!(name,
            "SSLeay" | "SSLeay_version" | "OpenSSLDie"
            | "string_to_hex" | "hex_to_string"
            | "explicit_bzero" | "freezero" | "recallocarray" | "reallocarray"
            | "strtonum"
        ) {
            return true;
        }
        // DNS resolver (libresolv)
        if name.starts_with("res_") || name.starts_with("dn_") {
            return true;
        }
        // GSSAPI / Kerberos
        if name.starts_with("gss_") || name.starts_with("krb5_") {
            return true;
        }
        // ncurses / panel / termcap
        if name.starts_with("w") && (name.contains("$NCURSES") || name.ends_with("$NCURSES60")) {
            return true;
        }
        // ncurses term management
        if name.starts_with("set_curterm") || name.starts_with("del_curterm") {
            return true;
        }
        // proc info
        if name.starts_with("proc_") {
            return true;
        }
        // Spotlight / Metadata framework
        if name.starts_with("MD") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // Spotlight private SPI with double-underscore prefix (__MDQuery*, __MDItem*)
        if name.starts_with("_MD") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase() {
            return true;
        }
        // Spotlight constant strings (kMDItemPath, kMDQueryDidFinishNotification, …)
        if name.starts_with("kMD") {
            return true;
        }
        // IOKit
        if name.starts_with("IO") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // Security framework
        if name.starts_with("Sec") || name.starts_with("CSSM") {
            return true;
        }
        // CSSM GUID data symbols (gGuidAppleCSP, gGuidAppleX509TP, …) from Security.framework
        if name.starts_with("gGuidApple") {
            return true;
        }
        // SystemConfiguration
        if name.starts_with("SC") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // NSPasteboard constants
        if name.starts_with("NS") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // AuthKit / AppleIDSession
        if name.starts_with("AK") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase() {
            return true;
        }
        // mach_port_ / mach_voucher_ / task_policy_ prefixes
        if name.starts_with("task_policy") {
            return true;
        }
        matches!(
            name,
            // I/O — stdio
            "printf" | "puts" | "putchar" | "fprintf" | "snprintf" | "sprintf"
            | "vprintf" | "vfprintf" | "vsnprintf" | "vsprintf" | "asprintf"
            | "fputs" | "fwrite" | "fread" | "fflush"
            | "fopen" | "fclose" | "freopen" | "feof" | "ferror" | "fputc"
            | "fgetc" | "getc" | "ungetc" | "fgets" | "rewind" | "fseek" | "ftell"
            | "fileno" | "open_memstream" | "sscanf" | "fscanf"
            | "__stderrp" | "__stdoutp" | "__stdinp"
            | "__sprintf_chk" | "__snprintf_chk" | "__fprintf_chk"
            // Additional stdio
            | "putc" | "fdopen" | "getline" | "getdelim" | "getchar" | "getlogin"
            | "pclose" | "tmpfile" | "setvbuf" | "setbuf" | "clearerr"
            | "dprintf"
            // Memory
            | "malloc" | "calloc" | "realloc" | "reallocf" | "free"
            | "memcpy" | "memset" | "memmove" | "memcmp" | "memchr" | "memset_s"
            | "__memcpy_chk" | "__memmove_chk" | "__memset_chk"
            | "bzero" | "bcopy" | "munmap" | "madvise" | "posix_madvise"
            | "malloc_size" | "malloc_good_size"
            | "mprotect" | "mlock" | "munlock" | "mlockall" | "munlockall"
            // Strings
            | "strlen" | "strnlen" | "strcmp" | "strncmp" | "strcasecmp" | "strncasecmp"
            | "strcpy" | "strncpy" | "strchr" | "strrchr" | "strstr" | "strcat" | "strncat"
            | "strerror" | "strerror_r" | "strdup" | "strndup" | "strlcpy" | "strlcat"
            | "strsignal" | "strsep" | "strtok" | "strtok_r"
            | "__strcpy_chk" | "__strncpy_chk" | "__strcat_chk"
            | "__strlcpy_chk" | "__strlcat_chk"
            | "__strncat_chk"
            | "index" | "rindex"
            // Additional string
            | "strspn" | "strcspn" | "strcoll" | "strxfrm"
            // Conversion
            | "atoi" | "atol" | "atof" | "strtol" | "strtoul" | "strtoll" | "strtoull"
            | "strtod" | "strtof"
            | "strtold" | "strtoimax" | "strtoumax"
            // ctype / locale
            | "_DefaultRuneLocale" | "__maskrune" | "setlocale"
            | "tolower" | "toupper" | "__tolower" | "__toupper"
            | "isalpha" | "isdigit" | "isspace" | "isalnum"
            | "isupper" | "islower" | "isprint" | "isxdigit" | "iscntrl" | "ispunct"
            | "isascii" | "toascii" | "isblank"
            // File I/O (write is already listed in stdio above)
            | "open" | "lseek" | "pread" | "pwrite" | "creat"
            | "fstat" | "stat" | "lstat" | "access" | "unlink" | "rename" | "remove"
            | "mkdir" | "rmdir" | "chdir" | "fchdir" | "getcwd"
            | "opendir" | "readdir" | "closedir" | "rewinddir"
            | "dup" | "dup2" | "pipe"
            | "mktemp" | "mkstemp" | "mkdtemp" | "mkstemps"
            | "realpath$DARWIN_EXTSN" | "realpath"
            | "utimensat" | "futimens"
            | "utimes" | "lutimes" | "futimes"
            | "fsync" | "fdatasync"
            | "ftruncate" | "truncate"
            | "chmod" | "fchmod" | "chown" | "fchown"
            | "umask"
            | "statfs" | "fstatfs"
            | "readlink" | "symlink" | "link"
            // File copy
            | "copyfile" | "copyfile_state_alloc" | "copyfile_state_free"
            // Path utilities
            | "basename" | "dirname" | "glob" | "globfree" | "fnmatch"
            // Device/file control
            | "ioctl" | "flock" | "mkfifo"
            // Scatter/gather I/O
            | "writev" | "readv"
            // *at() directory-relative operations
            | "fstatat" | "openat" | "mkdirat" | "unlinkat" | "renameat"
            | "faccessat" | "readlinkat" | "linkat" | "symlinkat"
            | "fchmodat" | "fchownat"
            // Dynamic loading
            | "dlopen" | "dlclose" | "dlsym" | "dlerror" | "dladdr"
            // Process
            | "_exit" | "abort" | "getenv" | "getpid" | "getppid"
            | "isatty" | "atexit" | "_NSGetExecutablePath" | "_NSGetEnviron"
            | "_Exit" | "environ" | "rand" | "srand" | "rand_r"
            | "gethostname" | "getentropy"
            | "sched_yield"
            | "qos_class_self" | "qos_class_main"
            | "random" | "srandom" | "srandomdev"
            | "system"
            // Environment mutation
            | "setenv" | "unsetenv" | "putenv"
            // Argument parsing
            | "optarg" | "optind" | "optopt"
            | "getuid" | "geteuid" | "getgid" | "getegid"
            | "fork" | "execv" | "execve" | "execvp" | "execl" | "execlp" | "execle"
            | "posix_spawn" | "posix_spawn_file_actions_init"
            | "posix_spawn_file_actions_destroy"
            | "posix_spawn_file_actions_adddup2"
            | "posix_spawn_file_actions_addopen"
            | "posix_spawn_file_actions_addclose"
            | "posix_spawnattr_init" | "posix_spawnattr_destroy"
            | "posix_spawnattr_setflags"
            | "posix_spawn_file_actions_addchdir_np"
            | "posix_spawnattr_setbinpref_np"
            | "posix_spawnattr_setpgroup"
            | "posix_spawnattr_setsigdefault" | "posix_spawnattr_setsigmask"
            | "posix_spawnattr_set_qos_class_np"
            | "posix_spawn_file_actions_addinherit_np"
            | "posix_spawnp"
            | "wait" | "waitpid" | "wait4"
            | "kill" | "raise"
            | "sigprocmask" | "pthread_sigmask"
            | "alarm" | "setsid"
            | "getprogname" | "setprogname"
            | "confstr" | "getpagesize"
            | "getpriority" | "setpriority" | "getsid"
            | "imaxabs"
            // Stack protection
            | "__stack_chk_fail" | "__stack_chk_guard"
            | "__chkstk_darwin"
            // libSystem thread-state data variable
            | "__is_threaded" | "___is_threaded"
            // Assertion / error
            | "__assert_rtn" | "__error"
            // BSD error/warning
            | "err" | "errx" | "warn" | "warnx"
            | "vwarn" | "vwarnx" | "verr" | "verrx"
            | "perror"
            // C++ ABI
            | "__cxa_atexit" | "__cxa_guard_acquire" | "__cxa_guard_release"
            | "__cxa_pure_virtual"
            // Time
            | "time" | "gettimeofday" | "clock_gettime"
            | "usleep" | "nanosleep" | "sleep"
            | "gmtime" | "gmtime_r" | "localtime" | "localtime_r"
            | "mktime" | "timegm" | "difftime"
            | "strftime" | "strftime_l" | "strptime" | "strptime_l"
            | "clock_gettime_nsec_np" | "clock_getres"
            | "asctime" | "asctime_r" | "ctime" | "ctime_r"
            | "tzset" | "daylight" | "timezone" | "tzname"
            // Sorting / searching
            | "qsort" | "bsearch"
            // pthread
            | "pthread_create" | "pthread_join" | "pthread_self"
            | "pthread_mutex_init" | "pthread_mutex_lock"
            | "pthread_mutex_unlock" | "pthread_mutex_destroy"
            | "pthread_mutex_trylock"
            | "pthread_mutexattr_init" | "pthread_mutexattr_destroy"
            | "pthread_mutexattr_settype"
            | "pthread_once" | "pthread_key_create" | "pthread_key_delete"
            | "pthread_getspecific" | "pthread_setspecific"
            | "pthread_cond_init" | "pthread_cond_wait" | "pthread_cond_timedwait"
            | "pthread_cond_signal" | "pthread_cond_broadcast"
            | "pthread_cond_destroy"
            | "pthread_rwlock_init" | "pthread_rwlock_rdlock"
            | "pthread_rwlock_wrlock" | "pthread_rwlock_unlock"
            | "pthread_rwlock_destroy"
            | "pthread_attr_init" | "pthread_attr_destroy"
            | "pthread_attr_setdetachstate" | "pthread_attr_getdetachstate"
            | "pthread_attr_setstacksize" | "pthread_attr_getstacksize"
            | "pthread_detach" | "pthread_cancel" | "pthread_exit" | "pthread_equal"
            | "pthread_main_np" | "pthread_yield_np"
            // pthread extras
            | "pthread_condattr_init" | "pthread_condattr_destroy"
            | "pthread_condattr_setclock"
            | "pthread_getname_np" | "pthread_setname_np"
            | "pthread_getschedparam" | "pthread_setschedparam"
            | "pthread_set_qos_class_self_np" | "pthread_get_qos_class_np"
            | "pthread_attr_set_qos_class_np"
            | "pthread_rwlock_tryrdlock" | "pthread_rwlock_trywrlock"
            | "pthread_threadid_np" | "pthread_mach_thread_np"
            | "pthread_attr_setschedparam" | "pthread_attr_getschedparam"
            | "pthread_atfork"
            | "pthread_mutexattr_setpshared"
            // ObjC runtime
            | "objc_msgSend" | "objc_retain" | "objc_release"
            | "objc_autoreleasePoolPush" | "objc_autoreleasePoolPop"
            | "objc_autorelease"
            | "objc_getClass" | "objc_alloc" | "objc_alloc_init"
            | "objc_opt_self" | "objc_storeStrong" | "objc_opt_new"
            | "objc_opt_class" | "objc_opt_respondsToSelector"
            | "objc_opt_isKindOfClass"
            | "objc_msgSendSuper2"
            | "objc_setProperty_nonatomic" | "objc_setProperty_nonatomic_copy"
            | "objc_setProperty_atomic" | "objc_setProperty_atomic_copy"
            | "objc_getProperty" | "objc_copyStruct"
            | "objc_begin_catch" | "objc_end_catch"
            | "objc_exception_throw" | "objc_exception_rethrow"
            | "objc_terminate"
            | "objc_exception_tryEnter" | "objc_exception_tryExit"
            | "objc_exception_extract" | "objc_exception_match"
            | "NSSetUncaughtExceptionHandler" | "NSGetUncaughtExceptionHandler"
            | "objc_enumerationMutation"
            | "object_getClass" | "object_setClass"
            // ARC runtime
            | "objc_retainAutorelease" | "objc_retainAutoreleasedReturnValue"
            | "objc_autoreleaseReturnValue" | "objc_unsafeClaimAutoreleasedReturnValue"
            | "objc_retainAutoreleaseReturnValue"
            | "objc_lookUpClass" | "objc_allocWithZone"
            | "objc_retainBlock" | "objc_loadWeakRetained"
            | "objc_initWeak" | "objc_destroyWeak" | "objc_storeWeak"
            | "objc_moveWeak" | "objc_copyWeak"
            | "objc_setAssociatedObject" | "objc_getAssociatedObject"
            | "objc_removeAssociatedObjects"
            | "_objc_empty_cache"
            // ObjC runtime introspection
            | "class_getName" | "class_getSuperclass" | "class_isMetaClass"
            | "class_getInstanceSize" | "class_getInstanceVariable"
            | "class_getInstanceMethod" | "class_getMethodImplementation"
            | "class_getProperty" | "class_respondsToSelector"
            | "class_addMethod" | "class_replaceMethod"
            | "class_copyMethodList" | "class_copyIvarList"
            | "class_copyPropertyList" | "class_createInstance"
            | "sel_getName" | "sel_registerName" | "sel_isEqual"
            | "method_getName" | "method_getTypeEncoding"
            | "method_getImplementation" | "method_setImplementation"
            | "method_getNumberOfArguments" | "method_exchangeImplementations"
            | "ivar_getName" | "ivar_getOffset" | "ivar_getTypeEncoding"
            | "property_getName" | "property_getAttributes"
            | "protocol_getName" | "protocol_isEqual"
            | "protocol_conformsToProtocol" | "protocol_copyProtocolList"
            | "protocol_copyMethodDescriptionList" | "protocol_getMethodDescription"
            | "protocol_copyPropertyList" | "protocol_getProperty"
            | "objc_getProtocol" | "objc_allocateProtocol" | "objc_registerProtocol"
            | "class_conformsToProtocol" | "class_copyProtocolList"
            | "class_addProtocol"
            | "objc_allocateClassPair" | "objc_registerClassPair"
            | "objc_disposeClassPair" | "objc_duplicateClass"
            | "class_addIvar" | "class_addProperty"
            // Blocks runtime
            | "_Block_copy" | "_Block_release"
            | "_Block_object_assign" | "_Block_object_dispose"
            | "_NSConcreteStackBlock" | "_NSConcreteGlobalBlock"
            // GCD
            | "dispatch_once" | "dispatch_once_f"
            | "dispatch_sync" | "dispatch_async"
            | "dispatch_apply"
            | "dispatch_queue_create" | "dispatch_get_main_queue"
            | "dispatch_get_global_queue"
            | "_dispatch_main_q"
            // GCD extras
            | "dispatch_group_create" | "dispatch_group_async"
            | "dispatch_group_wait" | "dispatch_group_enter" | "dispatch_group_leave"
            | "dispatch_group_notify"
            | "_dispatch_queue_attr_concurrent"
            | "dispatch_queue_create_with_target"
            | "dispatch_semaphore_create" | "dispatch_semaphore_wait"
            | "dispatch_semaphore_signal"
            | "dispatch_data_create" | "dispatch_data_get_size"
            | "dispatch_data_apply" | "dispatch_data_create_map"
            | "_dispatch_data_empty"
            | "dispatch_time" | "dispatch_walltime"
            | "dispatch_after" | "dispatch_after_f"
            | "dispatch_barrier_sync" | "dispatch_barrier_async"
            | "dispatch_source_create" | "dispatch_source_set_event_handler"
            | "dispatch_source_set_cancel_handler" | "dispatch_source_cancel"
            | "dispatch_source_set_timer"
            | "dispatch_resume" | "dispatch_suspend"
            | "dispatch_release" | "dispatch_retain"
            | "dispatch_set_target_queue"
            | "dispatch_queue_get_label"
            | "dispatch_get_specific" | "dispatch_queue_set_specific"
            | "dispatch_async_f" | "dispatch_sync_f"
            | "dispatch_io_create" | "dispatch_io_read" | "dispatch_io_close"
            | "dispatch_io_set_high_water" | "dispatch_io_set_low_water"
            | "dispatch_block_create" | "dispatch_block_create_with_qos_class"
            | "dispatch_queue_attr_make_with_qos_class"
            // Math
            | "floor" | "ceil" | "round" | "sqrt" | "pow"
            | "log" | "log2" | "log10" | "exp" | "exp2"
            | "sin" | "cos" | "tan" | "fabs" | "fmod" | "modf"
            | "acos" | "asin" | "atan" | "atan2"
            | "cosh" | "sinh" | "tanh"
            | "erf" | "erfc"
            | "expf" | "exp2f" | "expm1"
            | "frexp" | "frexpf" | "ldexp" | "ldexpf"
            | "log1p" | "log2f" | "logb" | "logbf" | "logf" | "log10f"
            | "powf" | "sqrtf"
            | "floorf" | "ceilf" | "roundf" | "truncf" | "trunc"
            | "fma" | "fmaf" | "fmin" | "fmax" | "fminf" | "fmaxf"
            | "copysign" | "copysignf"
            | "remainder" | "remainderf"
            | "rint" | "rintf" | "lrint" | "lrintf" | "llrint" | "llrintf"
            | "lround" | "lroundf" | "llround" | "llroundf"
            | "nextafter" | "nextafterf"
            | "scalbn" | "scalbnf"
            | "cbrt" | "cbrtf"
            | "hypot" | "hypotf"
            | "sinf" | "cosf" | "tanf"
            | "acosf" | "asinf" | "atanf" | "atan2f"
            | "coshf" | "sinhf" | "tanhf"
            | "ilogb" | "ilogbf"
            | "nan" | "nanf"
            | "j0" | "j1" | "jn" | "y0" | "y1" | "yn"
            | "__sincos" | "__sincosf"
            | "__exp10" | "__exp10f"
            | "fabsf" | "fmodf" | "modff"
            // Random — now shimmed via RandomShims, but keep safe-list
            // entries for binaries that link directly.
            | "arc4random" | "arc4random_buf" | "arc4random_uniform"
            // Mach / VM
            | "mach_task_self" | "mach_task_self_" | "mach_absolute_time"
            | "mach_host_self" | "mach_port_deallocate" | "mach_error_string"
            | "mach_timebase_info"
            | "host_info"
            | "vm_allocate" | "vm_deallocate" | "vm_page_size" | "vm_page_mask"
            | "task_set_exception_ports"
            | "mach_thread_self" | "vm_remap" | "vm_protect" | "vm_read"
            | "mach_vm_region" | "mach_vm_region_recurse" | "mach_vm_deallocate"
            | "mach_port_space_basic_info" | "vm_kernel_page_size" | "vm_page_shift"
            | "mach_port_allocate" | "mach_port_insert_right" | "mach_port_mod_refs"
            | "mach_msg" | "mach_msg_send" | "mach_msg_receive"
            | "task_info" | "thread_info" | "task_threads"
            | "host_statistics" | "host_statistics64"
            | "host_processor_sets" | "host_processor_set_priv"
            | "processor_set_tasks_with_flavor"
            | "pid_for_task" | "bootstrap_port"
            | "semaphore_create" | "semaphore_destroy"
            | "semaphore_signal" | "semaphore_wait"
            // launchd / bootstrap / vproc
            | "launch_data_alloc" | "launch_data_copy" | "launch_data_free"
            | "launch_data_get_type"
            | "launch_data_dict_insert" | "launch_data_dict_lookup"
            | "launch_data_dict_remove" | "launch_data_dict_iterate"
            | "launch_data_dict_get_count"
            | "launch_data_array_set_index" | "launch_data_array_get_index"
            | "launch_data_array_get_count"
            | "launch_data_new_fd" | "launch_data_new_machport"
            | "launch_data_new_integer" | "launch_data_new_bool"
            | "launch_data_new_real" | "launch_data_new_string"
            | "launch_data_new_opaque"
            | "launch_data_set_fd" | "launch_data_set_machport"
            | "launch_data_set_integer" | "launch_data_set_bool"
            | "launch_data_set_real" | "launch_data_set_string"
            | "launch_data_set_opaque"
            | "launch_data_get_fd" | "launch_data_get_machport"
            | "launch_data_get_integer" | "launch_data_get_bool"
            | "launch_data_get_real" | "launch_data_get_string"
            | "launch_data_get_opaque" | "launch_data_get_opaque_size"
            | "launch_data_get_errno"
            | "launch_msg" | "launch_get_fd"
            | "bootstrap_look_up" | "bootstrap_check_in"
            | "bootstrap_register" | "bootstrap_create_service"
            | "bootstrap_create_server" | "bootstrap_subset"
            | "bootstrap_parent" | "bootstrap_unprivileged"
            | "bootstrap_status" | "bootstrap_strerror"
            | "bootstrap_look_up2" | "bootstrap_check_in2"
            | "bootstrap_look_up3" | "bootstrap_check_in3"
            | "bootstrap_register2" | "bootstrap_get_root"
            | "vproc_swap_integer" | "vproc_swap_complex"
            | "vproc_swap_string"
            | "vproc_transaction_begin" | "vproc_transaction_end"
            | "vproc_standby_begin" | "vproc_standby_end"
            | "_vproc_transaction_begin" | "_vproc_transaction_end"
            | "_vproc_transaction_count"
            | "_vproc_transactions_enable"
            | "_vproc_standby_begin" | "_vproc_standby_end"
            | "_vproc_standby_count" | "_vproc_standby_timeout"
            | "_vproc_pid_is_managed" | "_vproc_transaction_try_exit"
            | "_vproc_get_last_exit_status" | "_vproc_set_global_on_demand"
            | "_vproc_send_signal_by_label"
            | "_vproc_log" | "_vproc_log_error"
            | "vprocmgr_init" | "vprocmgr_strerror"
            // System info
            | "sysctl" | "sysctlbyname" | "sysctlnametomib"
            | "getrusage" | "uname" | "getloadavg"
            // Atomics / locks
            | "os_unfair_lock_lock" | "os_unfair_lock_unlock"
            | "os_unfair_lock_trylock"
            | "os_unfair_lock_lock_with_options"
            | "_os_nospin_lock_lock" | "_os_nospin_lock_unlock"
            | "_os_lock_type_unfair" | "os_lock_lock" | "os_lock_unlock"
            | "OSAtomicAdd32" | "OSAtomicIncrement32"
            | "OSAtomicDecrement32" | "OSAtomicAdd32Barrier"
            | "OSAtomicIncrement32Barrier" | "OSAtomicDecrement32Barrier"
            | "OSAtomicCompareAndSwap32" | "OSAtomicCompareAndSwap32Barrier"
            | "OSAtomicCompareAndSwapInt" | "OSAtomicCompareAndSwapIntBarrier"
            | "OSAtomicCompareAndSwapLong" | "OSAtomicCompareAndSwapLongBarrier"
            | "OSAtomicCompareAndSwapPtr" | "OSAtomicCompareAndSwapPtrBarrier"
            | "OSAtomicOr32" | "OSAtomicOr32Barrier"
            | "OSAtomicAnd32" | "OSAtomicAnd32Barrier"
            | "OSSpinLockLock" | "OSSpinLockUnlock" | "OSSpinLockTry"
            // Exception handling / unwinding
            | "_Unwind_Resume" | "__objc_personality_v0"
            | "__gxx_personality_v0"
            // TLS
            | "_tlv_bootstrap"
            | "_tlv_atexit"
            // Dyld introspection
            | "_dyld_image_count" | "_dyld_get_image_header"
            | "_dyld_get_image_name" | "_dyld_get_image_vmaddr_slide"
            | "_dyld_register_func_for_add_image"
            | "_dyld_get_shared_cache_range"
            | "dyld_for_each_installed_shared_cache"
            | "dyld_shared_cache_for_each_file"
            // Mach-O swap functions
            | "swap_fat_header" | "swap_fat_arch" | "swap_fat_arch_64"
            | "swap_mach_header" | "swap_mach_header_64"
            | "swap_segment_command" | "swap_segment_command_64"
            | "swap_section" | "swap_section_64"
            | "swap_nlist" | "swap_nlist_64"
            // Mach-O arch info
            | "NXGetArchInfoFromCpuType" | "NXGetArchInfoFromName"
            | "NXGetLocalArchInfo" | "NXGetAllArchInfos"
            // Backtrace
            | "backtrace" | "backtrace_symbols" | "backtrace_symbols_fd"
            // Network: socket/connect/bind/sendmsg/recvmsg handled by SocketShims
            | "sendfile"
            | "getpwnam_r" | "getpwuid_r"
            | "getpwnam" | "getpwuid" | "getgrnam" | "getgrgid"
            // Network extras — getsockname/getpeername handled by SocketShims
            | "getifaddrs" | "freeifaddrs" | "if_nametoindex"
            | "socketpair"
            // DNS / resolver
            | "gethostbyname" | "gethostbyname2" | "gethostbyaddr"
            | "herror" | "hstrerror"
            | "catopen" | "catgets" | "catclose"
            | "getprotobyname" | "getprotobynumber"
            | "getservbyname" | "getservbyport"
            | "in6addr_any" | "in6addr_loopback"
            // I/O multiplexing (kqueue/kevent handled by shim table)
            | "select" | "poll"
            // Network extras
            | "getaddrinfo" | "getnameinfo" | "freeaddrinfo" | "gai_strerror"
            | "inet_ntop" | "inet_pton"
            | "htonl" | "htons" | "ntohl" | "ntohs"
            // Wide-char / wmem
            | "wmemchr" | "wmemcpy" | "wmemmove" | "wmemset" | "wmemcmp"
            | "wcslen" | "wcscpy" | "wcsncpy"
            | "wcscmp" | "wcscoll" | "wcsstr" | "wcsrchr" | "wcschr"
            | "wcstod" | "wcstol" | "wcstoul" | "wcstoull" | "wcstoll"
            | "wcswidth" | "wcwidth"
            | "btowc" | "wctob" | "wctomb"
            | "mbtowc" | "mbrtowc" | "mbstowcs" | "mbsrtowcs" | "wcstombs"
            | "mbrlen" | "mblen" | "mbsinit"
            | "__mb_cur_max"
            | "___mb_cur_max"
            | "__collate_lookup_l"
            | "wcsdup" | "wcsnlen" | "wcsnrtombs" | "wcsrtombs" | "wcslcpy" | "wcslcat"
            | "fgetwc" | "fgetwln" | "fputwc" | "fputws" | "fwprintf" | "fwide"
            | "getwc" | "putwc" | "putwchar" | "ungetwc" | "swprintf" | "vswprintf" | "vfwprintf"
            | "iswblank" | "iswspace" | "iswprint" | "iswupper" | "iswlower"
            | "iswdigit" | "iswalpha" | "iswalnum" | "iswctype" | "iswpunct"
            | "towupper" | "towlower" | "wctype"
            // Misc POSIX
            | "_NSGetArgc" | "_NSGetArgv" | "_NSGetProgname"
            // xcselect (Xcode CLI shim support)
            | "xcselect_invoke_xcrun"
            // Terminal
            | "tcgetattr" | "tcsetattr" | "cfmakeraw"
            | "tgetent" | "tgetstr" | "tgetnum" | "tgetflag" | "tgoto" | "tputs"
            | "setupterm" | "tigetnum" | "tigetstr" | "tigetflag"
            // vfork
            | "vfork"
            // readline / libedit
            | "readline" | "add_history" | "rl_completion_matches"
            | "rl_attempted_completion_function" | "rl_line_buffer"
            | "rl_point" | "rl_readline_name" | "el_init" | "el_end"
            // Syslog
            | "openlog" | "closelog" | "syslog" | "vsyslog"
            | "syslog$DARWIN_EXTSN"
            // ncurses / curses
            | "initscr" | "initscr$NCURSES60" | "endwin" | "endwin$NCURSES60"
            | "newwin" | "newwin$NCURSES60" | "delwin" | "derwin" | "derwin$NCURSES60"
            | "stdscr" | "COLS" | "LINES"
            | "cbreak" | "noecho" | "nonl" | "intrflush" | "intrflush$NCURSES60"
            | "keypad" | "keypad$NCURSES60" | "meta"
            | "wmove" | "werase" | "wsyncup" | "wtimeout" | "wresize" | "wresize$NCURSES60"
            | "waddnstr" | "waddnstr$NCURSES60"
            | "wattr_on" | "wattr_on$NCURSES60" | "wattr_off" | "wattr_off$NCURSES60"
            | "wgetch" | "wgetch$NCURSES60"
            | "doupdate" | "doupdate$NCURSES60"
            | "getmaxx" | "getmaxy"
            // panel
            | "new_panel" | "del_panel" | "show_panel" | "hide_panel"
            | "move_panel" | "top_panel" | "update_panels"
            // misc system
            | "memorystatus_control" | "humanize_number"
            | "SessionGetInfo"
            | "endpwent" | "setpwent" | "getpwent"
            | "setegid" | "seteuid" | "setgid" | "setgroups"
            // setjmp / longjmp
            | "setjmp" | "longjmp" | "_setjmp" | "_longjmp"
            | "sigsetjmp" | "siglongjmp"
            // Regex (POSIX) — all shimmed in PosixExtras for struct bridging
            // Sorting / directory scanning extras
            | "qsort_b" | "qsort_r" | "heapsort" | "mergesort"
            | "heapsort_b" | "mergesort_b"
            | "alphasort" | "scandir" | "scandir_b" | "scandir$INODE64"
            | "alphasort$INODE64"
            // Fortified extras
            | "__vsnprintf_chk" | "__vsprintf_chk"
            // Memory extras
            | "memset_pattern16" | "memset_pattern4" | "memset_pattern8"
            | "memmem"
            | "msync" | "mincore"
            | "malloc_default_zone" | "malloc_zone_statistics"
            | "malloc_zone_malloc" | "malloc_zone_calloc"
            | "malloc_zone_realloc" | "malloc_zone_free"
            | "malloc_zone_from_ptr" | "malloc_create_zone"
            | "malloc_destroy_zone" | "malloc_set_zone_name"
            | "malloc_zone_memalign" | "malloc_zone_batch_malloc"
            | "posix_memalign" | "aligned_alloc" | "valloc"
            // UUID
            | "uuid_clear" | "uuid_copy" | "uuid_compare"
            | "uuid_generate" | "uuid_generate_random" | "uuid_generate_time"
            | "uuid_is_null" | "uuid_parse"
            | "uuid_unparse" | "uuid_unparse_upper" | "uuid_unparse_lower"
            // Compiler helpers
            | "__progname" | "__udivti3" | "__umodti3"
            | "__divti3" | "__modti3"
            | "__muloti4" | "__multi3"
            | "__darwin_check_fd_set_overflow"
            // BSD string extras
            | "vasprintf" | "strpbrk" | "fgetln"
            | "strcasestr"
            | "dirname_r" | "basename_r"
            // File/dir extras
            | "mkpath_np" | "clonefile"
            | "fopen$DARWIN_EXTSN"
            | "fts_open" | "fts_read" | "fts_close" | "fts_set" | "fts_children"
            | "nftw" | "ftw"
            | "setlinebuf"
            | "fmtcheck"
            | "fseeko" | "ftello"
            | "flockfile" | "funlockfile" | "ftrylockfile"
            | "getc_unlocked" | "putc_unlocked"
            // Locale
            | "nl_langinfo" | "nl_langinfo_l" | "localeconv"
            | "newlocale" | "uselocale" | "freelocale" | "duplocale"
            | "querylocale"
            | "strcoll_l" | "strxfrm_l" | "strtod_l" | "strtol_l"
            | "wcstod_l" | "wcstol_l"
            // BSD/Darwin extras
            | "os_variant_is_darwinos" | "os_variant_has_internal_content"
            | "readpassphrase" | "getpass"
            | "getsectiondata" | "getsegmentdata"
            | "getattrlist" | "setattrlist" | "getattrlistbulk"
            | "getxattr" | "setxattr" | "listxattr" | "removexattr"
            | "fgetxattr" | "fsetxattr" | "flistxattr" | "fremovexattr"
            | "strmode" | "getmode" | "setmode"
            | "user_from_uid" | "group_from_gid"
            | "lchflags" | "chflags" | "fchflags"
            | "lchown" | "lchmod"
            | "strtofflags" | "fflagstostr"
            | "rpmatch"
            | "strtoq" | "strtouq" | "strtonum"
            | "errc" | "warnc" | "verrc" | "vwarnc"
            | "expand_number" | "devname"
            | "compat_mode"
            | "fsctl" | "gethostuuid"
            | "kdebug_trace" | "kdebug_trace_string"
            | "analytics_send_event_lazy"
            // NSLog and Foundation C API
            | "NSLog"
            | "NSSearchPathForDirectoriesInDomains"
            | "NSArgumentDomain"
            | "NSFileType" | "NSFileTypeSymbolicLink"
            | "NSTemporaryDirectory" | "NSHomeDirectory"
            // CF extras
            | "CFBundleCopyInfoDictionaryForURL" | "CFBundleCopyExecutableURL"
            | "CFBundleCreate" | "CFBundleGetValueForInfoDictionaryKey"
            | "CFDataAppendBytes" | "CFDataCreateMutable"
            | "CFStringHasPrefix" | "CFStringHasSuffix"
            | "CFURLCopyLastPathComponent"
            | "CFNumberIsFloatType"
            | "CFRunLoopGetMain" | "CFRunLoopGetCurrent"
            | "CFRunLoopRun" | "CFRunLoopRunInMode" | "CFRunLoopStop"
            | "CFRunLoopTimerCreate" | "CFRunLoopAddTimer"
            | "CFAbsoluteTimeGetCurrent"
            | "CFStringCreateWithCString" | "CFStringGetCStringPtr"
            | "CFStringGetCString" | "CFStringGetLength"
            | "CFStringCreateMutable" | "CFStringAppend"
            | "CFRelease" | "CFRetain"
            | "CFAllocatorGetDefault"
            | "CFNotificationCenterGetLocalCenter"
            | "CFNotificationCenterAddObserver"
            | "CFNotificationCenterRemoveObserver"
            | "CFNotificationCenterPostNotification"
            | "CFReadStreamCreateWithFile" | "CFWriteStreamCreateWithFile"
            | "CFReadStreamOpen" | "CFReadStreamClose"
            | "CFWriteStreamOpen" | "CFWriteStreamClose"
            | "CFPropertyListCreateWithStream"
            | "CFPropertyListCreateWithData" | "CFPropertyListCreateData"
            | "CFPropertyListWrite"
            | "CFGetTypeID" | "CFCopyDescription" | "CFCopyTypeIDDescription"
            | "CFHash" | "CFEqual"
            // Security framework
            | "SecKeychainItemFreeContent" | "SecKeychainFindGenericPassword"
            | "SecKeychainAddGenericPassword"
            | "SecRandomCopyBytes"
            | "SecTrustGetCertificateCount" | "SecTrustEvaluateAsync"
            // Network security (sec_protocol_* / sec_trust_*)
            | "sec_protocol_metadata_get_negotiated_protocol"
            | "sec_protocol_options_add_tls_application_protocol"
            | "sec_protocol_options_set_max_tls_protocol_version"
            | "sec_protocol_options_set_min_tls_protocol_version"
            | "sec_protocol_options_set_verify_block"
            | "sec_trust_copy_ref"
            // CommonCrypto extras
            | "CCDigest" | "CCDigestCreate" | "CCDigestUpdate" | "CCDigestFinal"
            | "CCHmac" | "CCHmacCreate" | "CCHmacUpdate" | "CCHmacFinal"
            | "CCHmacInit"
            | "CCKeyDerivationPBKDF"
            | "cc_clear" | "ccdigest_init" | "ccdigest_update" | "ccsha256_di"
            | "ccsha1_di"
            // Compression library
            | "zError" | "zlibVersion" | "zlibCompileFlags"
            // FP environment
            | "feclearexcept" | "fetestexcept" | "fegetenv" | "fesetenv"
            | "feraiseexcept" | "fegetround" | "fesetround"
            | "__srget" | "__swbuf"
            // Shared memory
            | "shm_open" | "shm_unlink"
            // Signal extras
            | "sigemptyset" | "sigfillset"
            | "sigaddset" | "sigdelset" | "sigismember"
            | "sigwait" | "sigtimedwait"
            // Additional stdio
            | "popen"
            // Carbon / legacy Mac OS APIs (DeRez, Rez, sdef …)
            // Resource Manager
            | "Get1Resource" | "Get1IndResource" | "Get1NamedResource"
            | "ReleaseResource" | "CloseResFile" | "OpenResFile"
            | "CurResFile" | "Count1Resources" | "CountResources"
            | "AddResource" | "RemoveResource" | "WriteResource"
            | "UpdateResFile" | "UseResFile" | "HomeResFile"
            | "GetResInfo" | "SetResInfo" | "GetResAttrs" | "SetResAttrs"
            | "GetResFileAttrs" | "SetResFileAttrs"
            | "GetIndResource" | "GetNamedResource" | "GetResource"
            | "LoadResource" | "DetachResource" | "ChangedResource"
            | "HGetState" | "HSetState"
            // Carbon Handle / Ptr (Memory Manager)
            | "HLock" | "HLockHi" | "HUnlock" | "HPurge" | "HNoPurge"
            | "DisposeHandle" | "DisposePtr"
            | "NewHandle" | "NewHandleClear"
            | "NewPtr" | "NewPtrClear"
            | "GetHandleSize" | "SetHandleSize"
            | "MoveHHi" | "PtrToHand" | "HandToHand"
            // Carbon CoreServices miscellaneous  
            | "GetMacOSStatusErrorString" | "GetMacOSStatusCommentString"
            | "Debugger" | "DebugStr"
            // TextEncoding / TextUtils
            | "CharacterByteType" | "TUCharacterByteType"
            | "UppercaseText" | "LowercaseText" | "StripDiacritics"
            // Carbon fork utilities (HFS resource forks)
            | "OpenRFPerm" | "OpenRF"
        )
    }
}

impl SymbolResolver for StubResolver {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        self.resolve(symbol_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct CountingHostResolver {
        open_library_calls: AtomicUsize,
    }

    impl CountingHostResolver {
        fn open_library_calls(&self) -> usize {
            self.open_library_calls.load(Ordering::SeqCst)
        }
    }

    impl HostSymbolResolver for CountingHostResolver {
        fn resolve(&self, _name: &str) -> Option<*const ()> {
            None
        }

        fn resolve_in(&self, _handle: *mut (), _name: &str) -> Option<*const ()> {
            None
        }

        fn open_library(&self, _path: &str) -> Option<*mut ()> {
            self.open_library_calls.fetch_add(1, Ordering::SeqCst);
            Some(NonNull::<()>::dangling().as_ptr())
        }

        fn default_handle(&self) -> *mut () {
            NonNull::<()>::dangling().as_ptr()
        }
    }

    fn test_resolver() -> StubResolver {
        #[cfg(target_os = "macos")]
        let host: Arc<dyn HostSymbolResolver> =
            Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host: Arc<dyn HostSymbolResolver> =
            Arc::new(platform_linux::LinuxSymbolResolver::new());
        StubResolver::new(host)
    }

    #[test]
    fn safe_symbols_allowed() {
        assert!(StubResolver::is_safe_symbol("printf"));
        assert!(StubResolver::is_safe_symbol("puts"));
        assert!(StubResolver::is_safe_symbol("malloc"));
        assert!(StubResolver::is_safe_symbol("free"));
        // `exit` is now shimmed (not on safe-list) to bypass atexit handlers
        assert!(StubResolver::is_safe_symbol("_exit"));
        assert!(StubResolver::is_safe_symbol("$sSS10FoundationE19_bridgeToObjectiveCSo8NSStringCyF"));
        assert!(StubResolver::is_safe_symbol("swift_task_isCurrentExecutor"));
    }

    #[test]
    fn unsafe_symbols_blocked() {
        assert!(!StubResolver::is_safe_symbol("setuid"));
        assert!(!StubResolver::is_safe_symbol("chroot"));
        assert!(!StubResolver::is_safe_symbol("ptrace"));
    }

    #[test]
    fn preload_state_only_opens_libraries_once() {
        let preload_state = PreloadState::new();
        let host = CountingHostResolver::default();
        let libraries = ["/tmp/libA.dylib", "/tmp/libB.dylib", "/tmp/libC.dylib"];

        preload_state.preload(&host, &libraries);
        preload_state.preload(&host, &libraries);

        assert_eq!(host.open_library_calls(), libraries.len());
    }

    #[test]
    fn stub_resolver_finds_printf() {
        let resolver = test_resolver();
        let addr = resolver.resolve("_printf");
        assert!(addr.is_some(), "should resolve printf via dlsym");
        assert!(addr.unwrap() != 0, "printf address should be non-zero");
    }

    #[test]
    fn stub_resolver_strips_underscore() {
        let resolver = test_resolver();
        let addr = resolver.resolve("_puts");
        assert!(addr.is_some(), "should resolve _puts → puts");
    }

    #[test]
    fn stub_resolver_rejects_unsafe() {
        let resolver = test_resolver();
        let addr = resolver.resolve("_setuid");
        assert!(addr.is_none(), "should not resolve _setuid");
    }
}
