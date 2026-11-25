# Linux product research and decision register

Updated 2026-11-25. Governed by [ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md).
Linux, a contribution-first strategy and Rust interfaces/bridges are settled
product decisions. This register selects implementations within that direction.

Evidence in this update is source/document inspection. No new Darling build,
integrated Linux image, application qualification or enforcement experiment is
reported as completed. Existing receipts retain their original revisions and
scope. Each experiment records its exact inputs, commands, output, resource
use, unsupported behavior and pass/fail/blocked outcome.

## Decision order

| ID | Question | Priority / owner | Current disposition |
| --- | --- | --- | --- |
| RQ-01 | Which Darwin execution backend should ship? | First / compatibility lead | Evaluate complete upstream Darling worker against current Burdettie baseline; not adopted |
| RQ-02 | Which framework/object model can we reuse coherently? | First / runtime lead | Component and ABI inventory required; avoid combining incompatible runtimes |
| RQ-03 | How is owner policy enforced on Linux? | First / security and platform leads | Mechanisms identified; execution-route qualification open |
| RQ-04 | Which Linux image and desktop foundation? | Early / platform and UI leads | Existing Linux development workflows now; release image/session unselected |
| RQ-05 | How much developer tooling can we delegate? | Early / developer-experience lead | Upstream compiler/LSP/DAP approach selected; frontend and integration gates open |
| RQ-06 | Which applications and CPU routes define success? | First / compatibility and product leads | x86_64 same-ISA evaluation first; exact app versions remain open |
| RQ-07 | What first upstream contribution helps both projects? | Parallel / contribution owner | SDK discovery, native libDER tooling and shared tests are bounded candidates |
| RQ-08 | Where do Rust interfaces reduce risk and maintenance? | Continuous / runtime lead | Audit ownership, unsafe/FFI and process boundaries before integration |
| RQ-09 | Can we maintain and distribute the selected dependency set? | Before adoption / release lead | Per-component provenance, licensing and upgrade/patch cost review required |

## RQ-01 — Darwin execution over Linux

**Findings.** Burdettie's current runtime redirects supported imports through
Rust shims and Linux services. Its Mach message handler has three selected MIG
cases; bootstrap uses an in-process synthetic registry; raw interpreted ARM64
SVC remains unsupported. These are useful assets, not a complete Darwin kernel
ABI. [Resolver](../crates/macho-runtime/src/resolver.rs),
[Mach messages](../crates/libsystem-shims/src/darwin_stubs/mach_extras.rs),
[bootstrap](../crates/launchd/src/bootstrap.rs),
[SVC](../crates/arm64-translate/src/interp/exec_dp.rs).

Darling `90af44da21d314b9acd24de1d1272e4aba8b9cc6` combines a loader, modified
Darwin libraries and userspace `darlingserver`. Its current build configuration
supports x86_64 and aarch64; this is source evidence, not a Burdettie application
pass. The pinned server is `3604875f2d3fa46985e914650d23ce124f99ac27`.
[README](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/README.md),
[architecture configuration](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/CMakeLists.txt).

**Experiment A: complete upstream worker first.** Pin upstream and submodules,
check build/install capacity, and run its normal loader/library/server path on
Linux with a matching host and guest architecture. Establish an upstream
control run before adding a Rust Burdettie supervisor adapter. Use existing
source-owned C/Foundation fixtures and one named developer workload chosen
before testing. Compare the same declared behavior with our current runtime.

Pass requires reproducible inputs, expected output/errors, allocation/file/
thread behavior, truthful exit status, crash containment and termination/reaping
of the entire worker process tree. Record startup, memory and build costs.
If the fixed corpus requires broad private upstream changes, stop and record
the blocker; do not expand the experiment into another compatibility stack.

**Experiment B: direct adapter only if justified by A.** Darling's generated C
RPC client can be used in both its Darwin libraries and ELF loader, making a
Rust wrapper plausible. But the server tracks real Linux processes/threads,
reads/writes their memory, and participates in fork/exec/signal lifetimes. It is
not a drop-in `PlatformProvider` implementation or a demonstrated stable public
embedding API. [RPC](https://docs.darlinghq.org/internals/darlingserver/rpc.html),
[lifetimes and memory](https://docs.darlinghq.org/internals/darlingserver/threads-and-friends.html),
[current process memory access](https://github.com/darlinghq/darlingserver/blob/3604875f2d3fa46985e914650d23ce124f99ac27/src/process.cpp).

The bounded probe covers registration, one authoritative port namespace, a real
two-process message round trip, transferred rights, bounded out-of-line memory,
timeout/interruption and cleanup after death. Server-visible mappings must be
valid worker memory; no supervisor object pointers cross the boundary. Handle
or explicitly reject server-initiated mmap/munmap/mprotect requests and their
interruption/restart behavior. Reject unsupported fork/exec rather than inventing
success. If interpreter state needs
pervasive server changes, prefer the complete-runtime worker or another measured
upstream approach. Do not maintain two competing port/ObjC/process identities
inside one execution route.

**Direct traps remain separate.** Darling replaces calls through
`libsystem_kernel`; its documentation explicitly excludes software issuing raw
inline Darwin syscalls. Adoption cannot mark Burdettie's SVC gap closed.
[System-call emulation](https://docs.darlinghq.org/internals/basics/system-call-emulation.html).

## RQ-02 — PureDarwin library/framework reuse

**Findings.** PureDarwin is pinned for this inspection at
`0c1990c803c13b68cad4df3f6dc34ba4306fc789`. Its Objective-C, CoreFoundation and
CoreGraphics recipes depend on a coherent Darwin libSystem/dyld/ObjC stack.
Linux-hosted compilation does not make their output Linux-native. Start with
portable source and common tests; evaluate a whole framework only after its
dependency and object-ownership boundary is explicit.

Sources: [libobjc](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/libobjc.nix),
[CoreFoundation](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/corefoundation.nix),
[CoreGraphics](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/coregraphics.nix).

**Experiment.** Inventory the imports, TLS, allocator, exceptions, callbacks,
ObjC objects and OS services of one useful component. Build it through an
existing Linux backend where possible and run a shared semantic fixture.
Pass includes ownership/release and negative cases, not only exported symbols.
Stop if the adoption would require duplicating the underlying runtime without
measurable benefit. Use upstream Linux ports such as Swift's libdispatch for
their supported native APIs rather than re-porting an XNU-dependent variant.
Portable Swift Foundation APIs do not establish macOS Objective-C binary ABI
compatibility. [Linux dispatch](https://github.com/swiftlang/swift-corelibs-libdispatch/blob/20b4c8b2e168a20a661edf8ce9c2bcc6dc79793b/README.md),
[corelibs Foundation scope](https://github.com/swiftlang/swift-corelibs-foundation/blob/0c2951a362addd219d6496517fe366feb79d0bda/README.md).

## RQ-03 — Owner trust and Linux enforcement

**Findings.** Current Darling source says its sandbox profiles are not enforced.
Its code-signing policy and local trust-store work are useful reuse candidates,
but those modes are not Burdettie's complete authority contract. The loader also
uses `PR_SET_PTRACER_ANY` to accommodate server memory access across PID
namespaces; this requires evaluating same-user peers and server authority.
[Sandbox source](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/sandbox/sandbox.c),
[signing tests](https://github.com/darlinghq/darlingserver/blob/3604875f2d3fa46985e914650d23ce124f99ac27/tests/code-signing.md),
[loader](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/mldr/mldr.c).

**Experiment.** Apply the existing launch-policy contract to an actual Linux
worker, descendants and every privileged helper/server. Evaluate Landlock or a
suitable LSM, namespaces, cgroups, seccomp and authenticated brokers for their
specific roles. Query actual kernel capabilities. Seccomp alone is not a full
sandbox; parsing SBPL or accepting a sandbox call supplies no enforcement.
[Linux allocation](contracts/LINUX_BACKEND_ALLOCATION.md),
[Landlock](https://docs.kernel.org/userspace-api/landlock.html),
[seccomp](https://docs.kernel.org/userspace-api/seccomp_filter.html).

Pass includes real denied file/network/IPC/process operations through library
calls, raw Linux calls, inherited descriptors, descendants and broker/server
requests; cross-worker isolation; immutable admitted inputs; and inability to
change supervisor policy. Missing required controls refuse enforced startup.
Permissive/audit semantics must remain distinct. Code-signature verification,
owner trust and resource confinement receive separate results. Capability-based
IPC remains an implementation question; an in-process typed handle alone does
not constrain arbitrary guest machine code.

## RQ-04 — Linux image and desktop

**Finding.** We can keep existing Linux development workflows while selecting a
release image. Compare existing distribution packaging/image tooling, including
NixOS's image/module workflow and a conventional distribution assembled with
mkosi. Neither is selected here. Prefer the existing kernel/driver/session
ecosystem; do not create another package manager, init system or compositor to
satisfy the Rust preference. [NixOS manual](https://nixos.org/manual/nixos/stable/),
[mkosi](https://github.com/systemd/mkosi).

**Experiment.** Pin one minimal Linux VM configuration, boot the Burdettie
session, run native and compatibility workers, preserve a document across a
clean reboot, and recover from a failed update/configuration. Compare build
cost, update/recovery behavior, upstream package availability and policy control
support before choosing the release base. A Docker desktop demo does not close
boot/image or hardware acceptance.

Use the existing [winit/softbuffer backend](../crates/window-backend-linux/src/lib.rs)
under an established Wayland session first. Evaluate an existing desktop/toolkit
against our UI requirements before considering a new compositor. Qualification
covers resize, input/IME, clipboard, file dialogs, graphics and shutdown. A
compositor does not provide AppKit semantics; portals do not alone prove Linux
confinement. The exact desktop/toolkit and hardware matrix remain open.

## RQ-05 — Our developer experience over upstream tools

**Finding.** Cargo exposes versioned metadata and JSON build diagnostics; clangd
uses the real compilation context; LLDB-DAP separates engine and IDE frontend;
Swift supports Linux development with SwiftPM/SourceKit-LSP. The existing
`burdettie-xcode-tools` already invokes a host compiler for a narrow project
subset. Preserve it as an importer/adapter rather than expanding it into a full
compiler/debugger stack.
[Cargo](https://doc.rust-lang.org/cargo/reference/external-tools.html),
[clangd](https://clangd.llvm.org/design/compile-commands),
[LLDB-DAP](https://lldb.llvm.org/use/lldbdap.html),
[Swift platforms](https://www.swift.org/platform-support/),
[existing build frontend](../crates/burdettie-xcode-tools/src/xcodebuild.rs).

**Experiment.** One owned Rust project and one C project produce accurate
diagnostics, build Linux ELF, run tests, stop at a breakpoint and inspect a
variable using reused tools. Add a portable SwiftPM project with a matched
Swift/SourceKit-LSP/debugger toolchain as a separately qualified route. Decide
whether an existing editor integration or a standalone frontend best serves our
UX. Native debugging does not qualify debugging interpreted Mach-O code. See
[the developer plan](XCODE_PUBLIC_COMPATIBILITY_PLAN.md).

## RQ-06 — Application and architecture contract

**Decision.** Start evaluation on Linux x86_64 with matching thin x86_64 Mach-O
fixtures to limit independent variables. The current
[native loader path](../crates/macho-runtime/src/pipeline_native.rs) requests an
ARM64 slice for universal inputs; verify/fix slice selection before including
fat binaries in an x86_64 result. Retain ARM64/interpreter regressions;
additional CPU routes require their own records. An upstream build option is
not an application-support claim. Cross-ISA translation, emulated thread state
and guest debugging remain explicit scope choices.

**Experiment.** Freeze a small source-owned CLI/Foundation corpus, one native
developer workflow, and a document workflow. Stickies remains an Apple-authored
candidate only after exact version, slice, dependency and service inventory.
Do not silently substitute a native demo for an Apple application. Keep the
source-owned document fixture as a separate reproducible test. Pass includes
real edit/save/reopen/error behavior and exact bytes, not simply an open window.
Missing application inputs do not block native Linux integration or shared
library contributions. [M1 contract](milestones/M1_PLATFORM_SECURITY.md).

## RQ-07 — First upstream contribution

The strongest researched starting points are consistent PureDarwin SDK
discovery, a Linux-host libDER package/regression corpus, SBPL host diagnostics
and a shared framework fixture. The exact wrapper version mismatch and their
test gates are recorded in [UPSTREAM_STRATEGY](UPSTREAM_STRATEGY.md).

Before implementation, check current upstream history and open work, reproduce
the issue in the intended path, select the actual maintainer destination and
follow its contribution process. PureDarwin welcomes contributions but directs
newcomers toward Nix packages and requires coordination for certain existing
system-library changes. No acceptance or submission is claimed by this register.

## RQ-08 — Rust interface quality

**Experiment.** For each adopted boundary, record pointer/handle ownership,
buffer lengths, integer/layout conversion, callbacks, thread affinity, errors,
panic/unwind behavior and cleanup after cancellation/death. Prefer typed Rust
resources around upstream C APIs or explicit IPC messages; keep foreign object
pointers inside the worker that owns them. Use focused negative tests and fuzz
the newly exposed parser/IPC boundary when it addresses real untrusted inputs.
Do not mirror upstream implementations with redundant Rust code or redundant
tests. [Rust FFI guidance](https://doc.rust-lang.org/nomicon/ffi.html).

## RQ-09 — Maintainability and distribution

Before adopting an implementation, record its component licenses, source
provenance, build inputs, security/update process, upstream contribution route,
downstream patch cost and migration/recovery plan. Darling's main license and
its submodule licenses differ; PureDarwin also has per-file/component notices.
Do not assume a process boundary automatically resolves distribution terms.
No proprietary SDK/application redistribution is authorized by an open-source
component's license. [Darling licensing](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/README.md),
[PureDarwin notices](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/NOTICE).

Pass: a reproducible component set with understood obligations, a manageable
patch queue and a demonstrated upstream update. Fail/blocked is an explicit
outcome; it does not revive the withdrawn requirement to replace the whole OS.
