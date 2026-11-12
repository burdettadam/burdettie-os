# Darwin Loader Lab — Security, Platform Support & Known Limitations

> See [FEATURES.md](FEATURES.md) for the full feature index and
> [roadmap.md](roadmap.md) for what comes next.

---

## Security Model

The security model mirrors Apple's 8-gate pipeline from `execve()` to code
execution. Every gate is a Rust trait; current implementations are permissive
mocks that log decisions, enabling testing without requiring Apple signing
infrastructure.

```
Binary on disk
  │
  ▼
┌─────────────────────────────┐
│ Gate 1: QUARANTINE          │  Gatekeeper first-launch check
│ Trait: QuarantinePolicy     │  Mock: always passes
└────────────┬────────────────┘
  ▼
┌─────────────────────────────┐
│ Gate 2: CODE SIGNATURE      │  CMS signature + CDHash + page hash verification
│ Trait: CodeSignatureValidator│ Mock: accepts any signature or unsigned binary
└────────────┬────────────────┘
  ▼
┌─────────────────────────────┐
│ Gate 3: ENTITLEMENTS        │  Parse + evaluate entitlements plist
│ Trait: EntitlementEvaluator │  Mock: grants all requested entitlements
└────────────┬────────────────┘
  ▼
┌─────────────────────────────┐
│ Gate 4: AMFI                │  Kernel code-integrity enforcement
│ Trait: AmfiPolicy           │  Mock: always allows
└────────────┬────────────────┘
  ▼
┌─────────────────────────────┐
│ Gate 5: SIP RESTRICTION     │  CS_RESTRICT flag, protected paths
│ Trait: SipPolicy            │  Mock: no paths restricted, no env stripping
└────────────┬────────────────┘
  ▼
┌─────────────────────────────┐
│ Gate 6: HARDENED RUNTIME    │  CS_RUNTIME flag, library validation
│ Trait: HardenedRuntimePolicy│  Mock: no hardened restrictions
└────────────┬────────────────┘
  ▼
┌─────────────────────────────┐
│ Gate 7: LIBRARY VALIDATION  │  Per-dylib load-time team-ID check
│ Trait: LibraryValidator     │  Mock: all libraries pass
└────────────┬────────────────┘
  ▼
┌─────────────────────────────┐
│ Gate 8: SANDBOX             │  File access + IPC profile enforcement
│ Trait: SandboxPolicy        │  Mock: all file access allowed
└────────────┬────────────────┘
  ▼
  Code executes in process
```

All 8 traits are composed into a `TrustPolicy` struct injected at runtime
construction:

```rust
pub struct TrustPolicy {
    pub quarantine:         Box<dyn QuarantinePolicy>,
    pub code_signature:     Box<dyn CodeSignatureValidator>,
    pub entitlements:       Box<dyn EntitlementEvaluator>,
    pub amfi:               Box<dyn AmfiPolicy>,
    pub sip:                Box<dyn SipPolicy>,
    pub hardened_runtime:   Box<dyn HardenedRuntimePolicy>,
    pub library_validator:  Box<dyn LibraryValidator>,
    pub sandbox:            Box<dyn SandboxPolicy>,
}

impl TrustPolicy {
    pub fn permissive() -> Self { /* all mocks, nothing enforced */ }
    pub fn apple_default() -> Self { /* future */ }
}
```

Code signature verification (Gate 2) uses pure-Rust SHA-256 (`sha2` crate).
CDHash values match output from Apple's `codesign` tool.

---

## Platform Support Matrix

| Platform | Architecture | Status | Notes |
|---|---|---|---|
| macOS | ARM64 (Apple Silicon) | ✅ Full support | Primary development target |
| macOS | x86_64 (Intel) | ✅ Compiles and checks | x86_64 trampolines verified; `cargo check` passes |
| Linux | ARM64 | ✅ PAL complete, CI in progress | All PAL traits implemented; Docker image available; Linux CI wiring pending |
| Linux | x86_64 | 🔲 Planned | Requires interpreter for ARM64 → x86_64 translation; E3d JIT not yet built |

---

## Known Limitations

Several Foundation API gaps remain after Phase C9. These are the highest-impact
boundaries of current real-world CLI tool compatibility:

| Priority | API | Impact | Status |
|---|---|---|---|
| 1 | **NSURLSession** | HTTP client — REST APIs, file downloads, network requests | Not started |
| 2 | **NSKeyedArchiver / NSKeyedUnarchiver** | Binary object graph serialization (Xcode caches, preferences) | Partial — Apple keyed-archive decode exists; broader semantic parity remains incomplete |
| 3 | **NSStream / NSInputStream / NSOutputStream** | Stream-based I/O | Not started |
| 4 | **NSRunLoop selectors** | `run`, `runUntilDate:`, `runMode:beforeDate:` (event loop management) | Partial — timers and blocking exist; broader source and mode behavior remains incomplete |
| 5 | **NSXMLParser** | XML parsing (config files, plists) | Partial — basic parsing exists; wider parity remains incomplete |
| 6 | **NSCoder (protocol)** | Foundation archiving base protocol — required for NSKeyedArchiver | Partial — base protocol exists; broader delegation and semantic parity remain incomplete |

Of these, **NSURLSession** is the highest-impact gap. It is used by any CLI tool
that makes HTTP requests or downloads files, and it unlocks the largest remaining
category of real-world macOS software.

Additionally:
- Build-time XIB/NIB compilation is still incomplete, but runtime loading of compiled nib archives is now implemented; broad `.app` compatibility still depends on wider AppKit coverage.
- CoreGraphics / CoreAnimation are not implemented — GPU rendering goes through `wgpu` directly.
- Linux CI is not yet wired (Docker image exists; CI matrix pending).
- The ARM64 interpreter processes ~12–50µs per fixture; JIT translation (E3d) is not yet implemented.

---

## Reporting Security Vulnerabilities

**Please do not open a public GitHub issue for security vulnerabilities.**

If you discover a security vulnerability in burdettieOS, please report it
privately through one of the following channels:

- **GitHub Security Advisories**: Use the "Report a vulnerability" button on the
  [Security tab](https://github.com/burdettadam/burdettie-os/security/advisories/new)
  of the repository. This is the preferred channel.
- **Email**: If you cannot use GitHub Advisories, contact the maintainers
  directly via the email address on the maintainer's GitHub profile.

Please include:
- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept (no need to weaponize it)
- The version or commit hash you tested against

We aim to acknowledge reports within 72 hours and to coordinate a fix and
disclosure timeline with you. We follow a 90-day responsible disclosure window.
