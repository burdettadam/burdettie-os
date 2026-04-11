# BurdettieOS — Rust Replacement Targets

Track progress replacing macOS system services with cross-platform Rust implementations.

## Completed

| # | Target | Description | Status |
|---|--------|-------------|--------|
| — | `launchd` / systemd | Full in-process reimplementation of liblaunch/bootstrap/vproc | ✅ Done |
| — | XPC pipe handlers | 17 daemon handlers (analyticsd, diagnosticd, securityd, trustd, etc.) | ✅ Done |
| — | `os_log` | burdettie-log-pipeline: ring buffer → JSON/logfmt/text formatter → sink | ✅ Done |
| — | Apple cloud APIs | burdettie-services: 20+ endpoint categories via domain rewrite + HTTP intercept | ✅ Done |
| — | `plutil` | burdettie-plutil: native Rust CLI (lint/convert/insert/replace/remove/extract) | ✅ Done |
| — | ObjC runtime | objc-runtime: 131 shims, ARM64 trampolines | ✅ Done |
| — | libSystem.B.dylib | libsystem-shims: 443 shim functions, 37 modules | ✅ Done |
| — | CoreFoundation | core-foundation-lite + cf-lite/cf_core | ✅ Done |
| — | Foundation | cf-lite: 193 classes, 867 selectors, 2001 shim functions | ✅ Done |
| — | CommonCrypto | Real AES CBC/ECB via ring | ✅ Done |
| — | Security.framework | trust-eval: real X.509/webpki, ECDSA P-256 | ✅ Done |
| — | LibreSSL bundle | Native Docker multi-stage build, 67/67 tests | ✅ Done |
| — | ICU (`libicucore`) | icu-shims: ICU4X-backed collation/datetime (~48%) | ✅ Done |

## In Progress

| # | Target | Description | Effort | Impact | Status |
|---|--------|-------------|--------|--------|--------|
| 4 | IOKit (power/device) | IOPMAssertion, IOServiceMatching, IORegistryEntry → Linux sysfs/upower | Medium | High | ✅ Done |
| 1 | `configd` / SCDynamicStore | Network state + change keys via Linux sysfs/procfs/resolv.conf | Medium | High | ✅ Done |

## Tier 1 — High impact, moderate effort

| # | Target | Description | Effort | Impact | Status |
|---|--------|-------------|--------|--------|--------|
| 1 | `configd` / SCDynamicStore | Network change notifications via Linux netlink (RTMGRP_LINK, RTMGRP_IPV4_IFADDR) | Medium | High | ✅ Done |
| 2 | `distnoted` / distributed notifications | Cross-process pub/sub via Unix domain sockets or inotify | Medium | High | ✅ Done |
| 3 | `mDNSResponder` / Bonjour | dns_sd.h API (Browse/Resolve/Register) via mdns/zeroconf crate | Medium | High | ✅ Done |

## Tier 2 — Medium impact, focused effort

| # | Target | Description | Effort | Impact | Status |
|---|--------|-------------|--------|--------|--------|
| 5 | `opendirectoryd` | User/group resolution via Linux NSS (getpwnam_r, getgrnam_r, getgrouplist) | Low | Medium | ✅ Done — sysinfo.rs +5 shims (mbr_uid/gid_to_uuid, mbr_uuid_to_id, mbr_check_membership, getgrouplist) |
| 6 | CoreAudio stub | Null audio backend so audio-aware apps can launch | Low | Medium | ✅ Done — core_audio.rs: 39 shims + 1 data symbol, 6 tests |
| 7 | `FSEvents` → inotify | File system event stream translation for file-watching tools | Medium | Medium | ✅ Done — fsevents.rs: 17 shims + 1 data symbol, 6 tests (inotify on Linux) |

## Tier 3 — Strategic, higher effort

| # | Target | Description | Effort | Impact | Status |
|---|--------|-------------|--------|--------|--------|
| 8 | Keychain Services | Real credential storage via secret-service D-Bus or encrypted SQLite | High | Medium | ⬜ Not Started |
| 9 | Network.framework | nw_connection/nw_listener state machine via tokio/socket2 | High | Medium | ⬜ Not Started |
| 10 | WindowServer / CoreDisplay | Headless compositor for virtual framebuffer rendering in CI | High | Low | ⬜ Not Started |

## Notes

- **Current test counts**: cf-lite 2910, libsystem-shims 585, Docker 580/580
- **IOKit coverage**: ~80% (130+ shims + 28 data symbols)
- **SC coverage**: ~75% (36 shims + 27 data symbols, real network state from sysfs/procfs)
- **Distributed notifications**: NSDistributedNotificationCenter (ObjC, 10+2 methods, 8 tests) + CFNotificationCenter (C-level, 8 shims, 9 tests) + notify.rs dirty-bit fix
- **Priority rationale**: Items 1-4 unblock real application workflows; items 5-7 prevent silent failures; items 8-10 enable advanced scenarios
