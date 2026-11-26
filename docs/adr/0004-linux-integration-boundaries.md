# ADR 0004: Linux integration boundaries and qualification experiments

Status: **accepted design boundaries and experiment order**, 2026-11-25.
No runtime backend, security capability, desktop session or release image is
qualified by this decision.

## Decision

Apply [ADR 0003](0003-linux-foundation-and-upstream-reuse.md) through explicit
integration boundaries. Reuse a complete compatibility runtime, an existing
compositor and an existing developer workbench wherever they meet the product
requirements. Rust owns the narrow interfaces, lifecycle and Burdettie product
logic. Linux remains the shipping foundation.

The research supports the boundaries below and selects bounded experiments.
Darling, labwc with Smithay Client Toolkit, Debian 13 with mkosi, and Eclipse
Theia remain **evaluation candidates**. Choosing an experiment does not adopt
its implementation for release or mark its required controls supported.

## Application and owner authority

Use one application/profile trust domain as the initial confinement and
lifecycle unit. For the candidate Darling route, that unit includes the whole
`darlingserver`, prefix, launchd, application workers, descendants and helpers.
Applications with different grants must not share an owner-wide runtime server
or writable prefix merely because they belong to the same Linux user.

The trusted Rust owner supervisor remains outside the domain. It authenticates
requests and owner decisions, binds admitted inputs, establishes the selected
controls, manages lifecycle and produces reports. Foreign application code and
its object pointers do not run inside that authority boundary. A compatibility
server's internal copying needed for an authorized operation does not grant
the application general process-inspection authority.

Share immutable runtime inputs across domains when their identity and access
are qualified. Keep writable application state, runtime sockets and policy
authority separate. An unrestricted writable overlay can shadow files in a
read-only lower image, so that layout alone does not satisfy immutable code
admission. The guest cannot replace an admitted dependency provider or change
the authoritative owner policy through a writable preference/trust store.

A distinct Linux service credential per domain and an equivalent qualified
LSM/process boundary are mechanisms to compare. A user-namespace UID mapping,
private prefix or signature verdict alone does not establish the required
isolation. The complete boundary must constrain direct syscalls, descriptor
transfer, descendants and services acting on the application's behalf.

See [Linux confinement research](../research/2026-11-25-linux-enforcement.md).

## Compatibility runtime and activation order

Establish a complete, pinned upstream **rootless Darling control run** before
adding the Burdettie adapter. Current upstream source supports that route; do
not begin by recreating a privileged launcher or directly embedding the server.
Rootless execution is a privilege/deployment choice, not a claim that Burdettie
policy is enforced.

After the control works, compare these explicit enforcement prototypes:

| Prototype | Qualification question |
| --- | --- |
| Complete rootless runtime with a post-setup restriction and activation barrier | Can trusted namespace/prefix assembly finish, then all required controls cover the server and workers before application initialization? |
| Existing outer launcher with Darling's non-root copy mode | Can reused outer filesystem/PID/network/lifecycle controls provide the boundary while the complete upstream runtime avoids inner mount setup? |

The second prototype derives isolation from its outer launcher; Darling's
non-root mode does not supply it. Each route has its own declared backend
identity, prerequisites, resource costs and evidence. Failure does not trigger
a retry under weaker controls or an undeclared execution mode.

Setup order is part of the design: applying filesystem restrictions before
required mount operations can prevent startup, while applying them after guest
initialization is too late. Qualify the actual handle set, process tree,
admission barrier and dynamic-load path. If integration needs a small general
upstream hook, prefer contributing that hook over replacing the runtime.

The restricted local workspace's available namespace/seccomp operations
do not compensate for blocked listener, memory-access or confinement
prerequisites. Host preflight failures remain explicit outcomes. Runtime
qualification requires a suitable existing Linux host and does not authorize
new paid infrastructure. The [retained AWS host](../research/2026-11-25-aws-host.md)
supplied selected prerequisites in its recorded SSM context, but its available
disk was below the existing reserve, so no runtime build started. See
[Darling runtime research](../research/2026-11-25-darling-runtime.md).

## One coherent guest ABI and framework closure

Evaluate a compatibility backend with its matched loader, Darwin libraries,
Objective-C runtime and framework providers. Treat their allocator, TLS,
exception, callback and object-lifetime conventions as one dependency closure.
Do not combine Burdettie's synthetic identities and upstream port/process/object
identities inside one route without an explicit, tested ownership contract.

Keep foreign objects inside the worker that owns their ABI. Cross-process
interfaces use bounded messages and opaque identifiers; in-process Rust FFI
wrappers specify layout, ownership, release, threading and unwind obligations.
Native Linux ELF libraries and developer tools are a separate target profile
from Darwin Mach-O compatibility. A portable Swift API is not evidence of a
macOS Objective-C binary ABI.

Preserve useful existing code and tests during comparison. Replace a private
implementation only after an upstream route satisfies the same declared
behavior, failure and recovery requirements. Direct server embedding remains
a later experiment justified by measured benefit, not a required architecture.

## GUI and desktop boundaries

Separate application windows, desktop shell clients and compositor authority.
Reuse an existing compositor and its supported protocols. Burdettie can own
Rust panel/launcher and other shell clients without implementing a new Wayland
compositor. Access to shell-level window enumeration/activation is authority
and must not be handed to every application by default.

Keep the application GUI event loop and its state on their owning thread.
Expose sendable commands/proxies across threads rather than a blanket unsafe
`Send + Sync` assertion over a thread-affine event loop. Consolidate and qualify
the existing event-delivery path before expanding custom GUI infrastructure.
An ordinary full-screen application window does not establish a desktop shell.

The first session experiment is **unmodified labwc plus a Rust layer-shell
client using Smithay Client Toolkit**, alongside ordinary application windows.
The first image experiment is **Debian 13 plus mkosi**. These choices bound the
next investigation; they do not settle the shipping compositor, toolkit,
distribution, image/update system or hardware matrix. NixOS and other maintained
foundations remain eligible if the evidence favors them.

## Developer product over a reusable workbench

Reuse the editor/workbench as well as compiler, language-server and debugger
implementations. Evaluate **Eclipse Theia** with a narrow Burdettie extension
and Rust CLI/service boundary. Keep project import, explicit target selection
and build/run orchestration usable through standard tool interfaces so they
are not permanently tied to one frontend. Small upstream-native UI extensions
are consistent with the Rust interface strategy.

The first native developer experiment has a separately declared **trusted
developer authority**. Workspace content, build scripts, terminals, extensions
and debug adapters can execute code and need an explicit trust decision.
Workbench workspace-trust UI does not implement the OS application contract.
Do not silently treat IDE subprocesses as confined application workers.

Preserve [launch-policy-v1](../contracts/launch-policy-v1.md), including its
deny-only process-inspection and runtime-generated-executable-memory controls.
Debugging and JIT-capable developer tools do not gain an exception merely
because Burdettie launches them. A future confined developer route needs a
separately reviewed capability contract and qualification. Audit/permissive
mode semantics remain unchanged; optional restrictions cannot be relabelled
as supervision to bypass those semantics.

See [desktop, image and developer-tool research](../research/2026-11-25-linux-desktop-devtools.md).

## PureDarwin reuse is evidence-driven

PureDarwin remains a partner for selected sources, packages, diagnostics and
shared tests. It is not a shipping kernel dependency. Portable compilation
does not establish semantic correctness or make a parser a trust verifier.

The Linux GCC libDER probe passed **28 of 30** selected cases, with **two
semantic failures**. That is a contribution/research result, not adoption into
the trusted signing path. Retain the failing cases, understand the intended
upstream semantics and qualify the affected behavior before use in admission.
CMS/chain validation and owner trust remain separate responsibilities.

The SDK metadata/path inconsistency is a focused toolchain contribution
candidate. Reproduce the affected caller path and keep discovery consistent
with its actual inputs; a corrected metadata report alone would not qualify
Darwin execution on Linux. Source and contribution-policy details remain in
[PureDarwin reuse research](../research/2026-11-25-puredarwin-reuse.md) and the
[upstream strategy](../UPSTREAM_STRATEGY.md).

## Consequences and promotion gates

The first integration work is a complete runtime adapter, a thread-affine GUI
boundary, a separate shell client and a workbench extension. It does not start
new kernel, compositor, editor, debugger or cryptographic-parser projects.
Carry only the smallest justified downstream changes, with upstream owners
and removal conditions recorded.

Promote a candidate only with evidence for its actual boundary: reproducible
application behavior, permitted and denied operations, initialization order,
dependency identity, lifecycle cleanup, update/recovery and maintenance cost.
Image configuration parsing, source inspection and narrow mechanism probes
remain scoped results. A booted image, an Apple application workflow, an IDE
debugging workflow and an enforced launch require separate qualification.

The [research register](../RESEARCH_QUESTIONS.md) and [roadmap](../roadmap.md)
track the remaining experiments. This ADR neither closes them by declaration
nor reopens the withdrawn requirement to replace the entire platform in Rust.
