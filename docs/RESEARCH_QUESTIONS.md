# Linux product research and decision register

Updated 2026-11-25. Governed by [ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md)
and [ADR 0004](adr/0004-linux-integration-boundaries.md).
Linux, a contribution-first strategy and Rust interfaces/bridges are settled
product decisions. This register selects implementations within that direction.

Evidence now includes source/document inspection and bounded executed probes:
upstream Darling launcher tests, Linux mechanisms, PureDarwin libDER and SDK
wrappers, native C development and mkosi configuration parsing. Their results
and limitations are recorded below. No complete Darling runtime, integrated
Linux image, IDE or enforced application route is qualified by those probes.
Existing receipts retain their original revisions and scope. Each experiment
records exact inputs, commands, output, resource use, unsupported behavior and
pass/fail/blocked outcome.

Research records: [Darling runtime](research/2026-11-25-darling-runtime.md),
[Linux enforcement](research/2026-11-25-linux-enforcement.md),
[PureDarwin reuse](research/2026-11-25-puredarwin-reuse.md),
[desktop and developer tools](research/2026-11-25-linux-desktop-devtools.md),
[existing AWS host preflight](research/2026-11-25-aws-host.md).

## Decision order

| ID | Question | Priority / owner | Current disposition |
| --- | --- | --- | --- |
| RQ-01 | Which Darwin execution backend should ship? | First / compatibility lead | Rootless core/system control first; selected prerequisites passed on an existing host, but disk reserve blocked the build; not adopted |
| RQ-02 | Which framework/object model can we reuse coherently? | First / runtime lead | Keep one coherent runtime; libDER portability passed but two semantic expectations failed, so trust-parser adoption is held |
| RQ-03 | How is owner policy enforced on Linux? | First / security and platform leads | Per-application whole-runtime domain selected; simple executable-memory filter disproved; full enforcement open |
| RQ-04 | Which Linux image and desktop foundation? | Early / platform and UI leads | Debian 13/mkosi and unmodified labwc/SCTK shell are prototype choices; image/session not qualified |
| RQ-05 | How much developer tooling can we delegate? | Early / developer-experience lead | Evaluate Theia workbench and upstream tools; native C probe passed, LSP/DAP/IDE gates open |
| RQ-06 | Which applications and CPU routes define success? | First / compatibility and product leads | x86_64 same-ISA evaluation first; exact app versions remain open |
| RQ-07 | What first upstream contribution helps both projects? | Parallel / contribution owner | SDK wrapper mismatch reproduced; libDER regressions/contract clarification precede adoption; no submissions |
| RQ-08 | Where do Rust interfaces reduce risk and maintenance? | Continuous / runtime lead | Thread-affine GUI driver/proxy and whole-runtime process boundary identified; implementation review remains open |
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

**New evidence.** Current source has rootless, non-root copy and relocatable
unprivileged-install paths. Do not assume a new privileged launcher is needed.
Eight upstream launcher test programs compiled locally; six passed and two
failed under observed local Unix-socket/namespace-ioctl restrictions.
These are host-level tests, not a complete runtime or application result.
[Detailed source and execution record](research/2026-11-25-darling-runtime.md).

A second, compiler-free probe on the existing AWS development host produced
positive Unix stream/datagram round trips, self `process_vm_readv`,
`PR_SET_PTRACER_ANY`, `NS_GET_USERNS` and namespace-creation observations. A
minimal Landlock ABI 8 child also allowed one file and denied another with
`EACCES`. These outcomes distinguish restrictions of the local research
environment from prerequisites available on another host; they do not qualify
Darling or a shipping security boundary. Disk preflight found 5,231,730,688
bytes free, below the 8 GiB reserve, so no build, dependency installation,
download or volume expansion was performed.
[Host context and limits](research/2026-11-25-aws-host.md).

**Experiment A: complete upstream worker first.** Pin upstream and submodules,
check build/install capacity, and start with the upstream core/system build
and matching x86_64 CLI fixtures. Establish the complete loader/library/server
control route before adding a Rust Burdettie supervisor adapter. Defer the
full framework build until this baseline works; then add Foundation and one
named developer workload. Compare the same declared behavior with our current
runtime. A restricted research host is not evidence that the upstream runtime
needs replacement. The next gate is a complete upstream run in a declared
environment with adequate existing capacity, preserving the resource bounds.

Pass requires reproducible inputs, expected output/errors, allocation/file/
thread behavior, truthful exit status, crash containment and termination/reaping
of the entire worker process tree. Record startup, memory and build costs.
If the fixed corpus requires broad private upstream changes, stop and record
the blocker; do not expand the experiment into another compatibility stack.

After the control run, compare rootless setup with a restriction/activation
barrier against an existing outer launcher with Darling's non-root copy mode.
Both need the whole-runtime domain and grant boundaries described in RQ-03.
Non-root copy mode is not itself a sandbox or a fallback that may silently
weaken an enforced request.

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

**Executed portable component probe.** The existing libDER C sources compiled
and ran on Linux, including sanitizer runs. Of 30 semantic expectations, 28
passed and two failed: BIT STRING unused-bit canonicality and constructed
SEQUENCE tag representation. Portability therefore does not establish a usable
trust parser. Hold adoption in the signature/entitlement path; prepare shared
regressions and resolve the API/encoding contract upstream before a Rust
wrapper is promoted. No memory-safety conclusion beyond the executed corpus
is implied. [libDER evidence](research/2026-11-25-puredarwin-reuse.md).

The [AWS preflight](research/2026-11-25-aws-host.md) did not run a library or
framework build because its free disk was below the reserved capacity. It
does not change these libDER findings or close the coherent-framework gate.

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

**Boundary decision.** One application/profile trust domain includes its
compatibility server, prefix, launchd, workers, descendants and helpers. Keep
the authenticated owner supervisor and authoritative policy outside that
domain. Share immutable runtime inputs, not an owner-wide mutable server or
prefix. Rootless namespaces and a same-UID socket check do not express
different application grants. Test distinct credentials or an equally
qualified process/LSM boundary, server-mediated requests and cross-domain
access. Writable overlays must not shadow admitted executable providers.
[ADR 0004](adr/0004-linux-integration-boundaries.md).

**Executed mechanism probes.** An additional seccomp filter denied one raw
syscall in a child and grandchild while allowing a control operation. Landlock,
Darling memory operations, named Unix sockets, cgroup delegation and a user
systemd manager were unavailable or blocked in the local restricted workspace. Namespace
creation succeeded, which does not qualify isolation. A small filter denied
RWX mappings and RW-to-RX protection changes but still allowed writable and
executable aliases of one memfd. No generated instructions were executed.
This disproves those two checks as a complete v1 generated-code denial; it
does not test systemd's complete implementation. The next prototype must
cover file/descriptor aliases and admitted loader transitions as well as direct
mapping calls. [Probe record](research/2026-11-25-linux-enforcement.md).

## RQ-04 — Linux image and desktop

**Prototype choice.** Evaluate Debian 13 x86_64 assembled with mkosi, following
the existing Debian integration family, plus an unmodified labwc compositor
and a separate Rust SCTK layer-shell client for our panel/launcher. The release
image/session remains open. NixOS remains a contender for its image/module and
generation/recovery workflows; measure actual runtime packaging and upgrade
cost before selecting it. Prefer the existing kernel/driver/session ecosystem;
do not create another package manager, init system or compositor to satisfy
the Rust preference. [Research comparison](research/2026-11-25-linux-desktop-devtools.md).

**Experiment.** Pin one minimal Linux VM configuration, boot the Burdettie
session, run native and compatibility workers, preserve a document across a
clean reboot, and recover from a failed update/configuration. Compare build
cost, update/recovery behavior, upstream package availability and policy control
support before choosing the release base. A Docker desktop demo does not close
boot/image or hardware acceptance.

The active native demo path is the host-backed
[`window-backend-darwin::GuiApp`](../crates/window-backend-darwin/src/lib.rs),
which has an application event handler. The separately named
[`WinitBackend`](../crates/window-backend-linux/src/lib.rs) polling path lacks
an event producer/pump; do not treat it as the qualified Linux path. Consolidate
event-loop ownership before expanding GUI infrastructure. Ordinary winit
windows, desktop layer-shell clients and the compositor are separate roles.

Qualification covers panel anchoring/space, launch/focus, multiple windows,
resize, input/IME, clipboard, file dialogs, accessibility and shutdown. Verify
privileged shell protocol access separately from restricted application
access. Labwc's feature scope and contribution policy constrain this prototype;
a workflow outside that scope can change the compositor choice without
justifying our own compositor. A compositor does not provide AppKit semantics;
portals do not alone prove Linux confinement.

**Executed image-tool probe.** Pinned mkosi accepted a Debian trixie x86_64
bootable-disk configuration and returned its dependency list. Package
resolution, a frozen snapshot/tools tree, image construction and boot were not
tested; required image/VM tools were absent. Configuration parsing does not
close the image/session gate. [Record and reproducer](research/2026-11-25-linux-desktop-devtools.md#executed-mkosi-configuration-probe).

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

**Frontend direction.** Evaluate Theia's existing workbench and extension
interfaces before implementing another editor, terminal, project tree or
extension host. Keep Burdettie import/target/build actions accessible through a
narrow Rust CLI/service and a portable extension where useful. Check the exact
extension API and upstream-tool versions; installability is not feature
qualification. Theia workspace trust is not the OS authority boundary. Keep
the initial native IDE/debugger experiment in an explicit trusted development
route rather than weakening launch-policy-v1 for Electron/JIT or inspection.
[Workbench comparison](research/2026-11-25-linux-desktop-devtools.md#4-reuse-the-workbench-as-well-as-the-compilers).

**Executed baseline.** GCC 14.2 built and ran a native ELF64 x86_64 C fixture,
then produced changed output after a source edit/rebuild. Invalid input yielded
a structured error with source position. Rust, LSP, DAP and Swift tools were
absent in the inspected host paths, so no IDE or debugging pass is claimed.
[Native probe](research/2026-11-25-linux-desktop-devtools.md#5-executed-native-compiler-probe-and-remaining-gates).

**Next experiment.** One owned Rust project and one C project produce accurate
diagnostics, build Linux ELF, run tests, stop at a breakpoint and inspect a
variable using reused tools. Add a portable SwiftPM project with a matched
Swift/SourceKit-LSP/debugger toolchain as a separately qualified route. Decide
whether an extension in an existing IDE or a composed Theia product best serves
our UX. Native debugging does not qualify debugging interpreted Mach-O code. See
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

The SDK wrapper mismatch is now reproduced narrowly: a fixture path override
named `MacOSX26.5.sdk` is reported by `--show-sdk-path`, while
`xcrun --show-sdk-version` still reports 11.3. This is a rendered shell-wrapper
test, with no SDK or complete Nix build. LibDER has a Linux build and
two failed semantic expectations, moving regressions/contract clarification
ahead of adopting it. SBPL diagnostics and shared framework fixtures remain
additional candidates. Scope and gates are recorded in
[UPSTREAM_STRATEGY](UPSTREAM_STRATEGY.md) and the
[PureDarwin evidence](research/2026-11-25-puredarwin-reuse.md).

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

**Concrete GUI boundary.** Keep winit's event loop on its owning thread and
use a typed command/EventLoopProxy interface for cross-thread work. The
current blanket unsafe `Send + Sync` on a backend containing that loop should
not become the model for new interfaces. Prefer foreign pointers inside the
worker that owns the upstream runtime, with bounded messages across the Rust
supervisor boundary. [Source findings](research/2026-11-25-linux-desktop-devtools.md#1-the-actual-gui-paths-differ-from-the-crate-names).

## RQ-09 — Maintainability and distribution

Before adopting an implementation, record its component licenses, source
provenance, build inputs, security/update process, upstream contribution route,
downstream patch cost and migration/recovery plan. Darling's main license and
its submodule licenses differ; PureDarwin also has per-file/component notices.
Do not assume a process boundary automatically resolves distribution terms.
No proprietary SDK/application redistribution is authorized by an open-source
component's license. [Darling licensing](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/README.md),
[PureDarwin notices](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/NOTICE).

Contribution fit is part of selection: labwc currently rejects AI-generated
contribution content, SCTK asks for general modular client changes, Theia
requires an Eclipse Contributor Agreement, and PureDarwin distinguishes
AI-assisted code from ordinary human-authored project communications. No
submission or agreement is implied by a reuse experiment. Review the current
component rules before preparing a contribution; do not solve an out-of-scope
feature by assuming an indefinite private fork. [Contribution strategy](UPSTREAM_STRATEGY.md).

Pass: a reproducible component set with understood obligations, a manageable
patch queue and a demonstrated upstream update. Fail/blocked is an explicit
outcome; it does not revive the withdrawn requirement to replace the whole OS.
