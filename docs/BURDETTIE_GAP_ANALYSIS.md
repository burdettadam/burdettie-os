# BurdettieOS vs Apple — Gap Analysis

> Generated 2026-04-06. Tracks delta between real Apple macOS subsystems and
> BurdettieOS emulation layer.

## System Architecture Comparison

| Apple Subsystem | BurdettieOS Equivalent | Coverage |
|---|---|---|
| dyld (Mach-O loading, chained fixups, shared cache) | macho-parse/map/exec/resolve/objc/shared-cache | ~100% |
| AMFI + code signing | trust-policy (8 gates, real CDHash, CMS chain) | ~100% |
| Rosetta 2 | arm64-translate (164 insn, JIT, block cache) | ~100% |
| ObjC runtime | objc-runtime (131 shims, trampolines, ARC, KVO) | ~98% |
| CoreFoundation | core-foundation-lite + cf-lite/cf_core | ~91% |
| libSystem.B.dylib | libsystem-shims (443 fns, 37 modules) | ~92% |
| XPC | xpc.rs + xpc_pipe.rs + xpc_transport.rs (~70 shims) | ~100% |
| GCD (libdispatch) | libdispatch-sys FFI + dispatch shims | ~100% |
| Security.framework | security.rs (~90 stubs) + trust-eval (real X.509) | ~65% |
| Foundation | cf-lite (193 classes, 867 selectors, 2001 shim fns) | ~43% selectors |
| CFNetwork | cfnetwork.rs (~45 stubs) | ~75% |
| ICU | icu-shims (50 fns, ICU4X real collation/locale/datetime) | ~48% |
| IOKit | iokit.rs (27 stubs) | ~14% |
| AppKit | app-kit-lite + cf-lite stubs (40 classes) | ~10% |
| os_log | os_log.rs (22 hooks) → burdettie-log-pipeline | ~100% |
| CommonCrypto | common_crypto.rs (real AES CBC/ECB) | ~100% |
| SystemConfiguration | system_configuration.rs (21 stubs, real reachability) | ~80% |

---

## Tier 1 — Internal Wiring Bugs (quick wins)

| # | Gap | Status |
|---|---|---|
| 1 | **Per-category status counters always 0** — `REQUESTS_TELEMETRY/CERTS/DNS/NOTARIZE/REPLAY_HIT` declared in `status.rs` but never incremented by any handler | ✅ DONE |
| 2 | **ConfigHandler is dead code** — SwupdateHandler claims `/api/v1/config` before ConfigHandler in router chain; ConfigHandler's richer plist never served | ✅ DONE |
| 3 | **`guest_binary` field always empty** — SQLite `requests.guest_binary` is `""` for all recorded HTTP events; can't correlate requests to tools | ✅ DONE |
| 28 | **Log sink File/Http declared but not implemented** — `LogSink::File` and `LogSink::Http` config options exist but pipeline only routes through `log` crate | ✅ DONE |

## Tier 2 — Missing Domain Handlers (16 domains → CatchallHandler)

| # | Domain(s) | Path | Handler | Status |
|---|---|---|---|---|
| 5 | `*.icloud.com`, `*.icloud-content.com`, `gateway.icloud.com`, `*.apple-cloudkit.com` | `/api/v1/icloud/*` | `ICloudHandler` — account state, CloudKit notarization (ticket lookup/submit), content tokens, gateway containers | ✅ DONE |
| 6 | `*.push.apple.com` | `/api/v1/push` | `PushHandler` — "not connected" (APNs is TCP/TLS, not HTTP) | ✅ DONE |
| 7 | `lcdn-registration/locator/suconfig.apple.com` | `/api/v1/cache/*` | `CacheHandler` — registration ack, empty peer list, caching config plist | ✅ DONE |
| 8 | `time.apple.com`, `time-macos.apple.com` | `/api/v1/time` | `TimeHandler` — Unix timestamp (NTP is UDP, not HTTP) | ✅ DONE |
| 9 | `diagassets.apple.com` | `/api/v1/diagnostics/assets` | `DiagnosticsHandler` — empty asset manifest plist | ✅ DONE |
| 10 | `oscdn/osrecovery.apple.com` | `/api/v1/recovery/*` | `RecoveryHandler` — empty recovery catalog plist | ✅ DONE |
| 11 | `*.itunes.apple.com`, `*.apps.apple.com`, `ppq.apple.com`, `*.mzstatic.com` | `/api/v1/content/*`, `/api/v1/apps/validate` | `ContentHandler` — store not available, enterprise validation accepted, mzstatic 404 | ✅ DONE |

## Tier 3 — Missing XPC Daemon Handlers ✅ DONE

All 7 XPC daemon handlers implemented in `crates/libsystem-shims/src/darwin_stubs/xpc_pipe.rs`.
17 total pipe handlers registered (2 launchd + 15 BurdettieOS daemons). 17/17 xpc_pipe tests pass.
Docker: 221/221 tools PASS, 0 FAIL, 0 CRASH.

| # | Service | What It Does on Apple | Status |
|---|---|---|---|
| 12 | `com.apple.securityd` | Keychain access, certificate trust evaluation | ✅ Returns errSecItemNotFound (-25300) |
| 13 | `com.apple.trustd` | Trust evaluation daemon (SecTrust XPC) | ✅ Returns kSecTrustResultProceed (1) |
| 14 | `com.apple.MobileFileIntegrity` | Runtime code signing enforcement | ✅ Returns valid=true, status=0 |
| 15 | `com.apple.coreservices.launchservicesd` | App registration, UTI database | ✅ Returns kLSApplicationNotFoundErr (-10814) |
| 16 | `com.apple.notifyd` | Darwin notification center (cross-process) | ✅ Returns token=1, status=0 |
| 17 | `com.apple.system.DirectoryService.membership_v1` | User/group membership resolution | ✅ Returns uid/gid + member=1 |
| 18 | `com.apple.syslog` | Unified log drain (legacy) | ✅ Returns success=true |

## Tier 4 — Shim Layer Gaps (partial ✅)

Actionable items fixed; remaining items deferred (high effort / low CLI impact).
401/401 libsystem-shims tests pass. Docker: 257/261 PASS, 0 FAIL, 4 CRASH (pre-existing find).

| # | Gap | Status |
|---|---|---|
| 19 | Keychain dict-key extraction | ✅ Fixed dual NSDictionary/NSConstantDictionary layout detection in `dict_value_for_sentinel` |
| 20 | Network.framework fully stubbed | ⏭ Deferred — high effort, low CLI impact (29 stubs already prevent symbol failures) |
| 21 | No `getaddrinfo` shim | ✅ Added shim with `burdettie_domain_map::rewrite_hostname()` + libc forward; also `freeaddrinfo`, `getnameinfo`, `gethostbyname`, `gethostbyname2` |
| 22 | No `res_query`/`res_search` shim | ✅ Added stubs returning -1 (SERVFAIL) to prevent DNS leaks; also `res_send`, `res_init`, `dn_expand` |
| 23 | IOKit at ~14% | ⏭ Deferred — low CLI impact (27 stubs sufficient for current test suite) |
| 24 | Swift metadata stubs incomplete | ⏭ Deferred — requires Xcode framework binaries for per-type-kind codegen |

## Tier 5 — Protocol Fidelity Gaps (partial ✅)

| # | Gap | Status |
|---|---|---|
| 25 | OCSP responder uses placeholder cert hashes | ✅ Now echoes CertID from request (issuerNameHash, issuerKeyHash, serialNumber); falls back to placeholder if request parsing fails |
| 26 | Notarization `signedTicket` is XML plist, not CMS | ✅ Changed to binary plist (matches Apple format); CMS wrapping deferred |
| 27 | DNS handler returns JSON, not wire-format `application/dns-message` | ⏭ Deferred — CLI tools use JSON path; wire-format encoder ~50 lines, low priority |
| 28 | Log sink File/Http declared but not implemented | ✅ DONE (see Tier 1) |

## Priority Matrix

| Priority | Gaps | Effort |
|---|---|---|
| **Quick wins** (< 1 hour each) | #1, #2, #3, #28 | Low |
| **Medium effort** (few hours each) | #12-13, #19, #21 | Medium |
| **Large effort** (days) | #24, #4 (os_log format decode), #26 | High |
| **Not needed for CLI tools** | #6, #7, #8, #10, #11, #20, #23 | N/A |
