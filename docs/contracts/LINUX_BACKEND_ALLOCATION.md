# Linux backend allocation

Strategy: [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and
[ADR 0004](../adr/0004-linux-integration-boundaries.md).
Status: **whole-runtime boundary selected; mechanisms and qualification open**,
2026-11-25.

This is the current product planning map for the 17 control families in
[launch-policy-v1](launch-policy-v1.md). It is not a runtime capability manifest
or evidence that any new Linux control is supported. Existing admission and
component tests retain their own scope. The historical
[PureDarwin allocation](BACKEND_ALLOCATION.md) does not qualify this backend.

## Boundary

The trusted Rust supervisor authenticates owner choices, binds immutable inputs
and grants, establishes required controls and owns worker lifecycle. Native
guest code remains untrusted machine code even when the loader is Rust.
Constrain the actual worker, descendants, native helpers and compatibility
server/brokers that can act for it. Start with a separate application/profile
trust domain containing the complete runtime: its server, prefix, launchd,
workers and helpers. Do not share an owner-wide mutable server/prefix across
applications with different grants. The owner supervisor and authoritative
policy/trust state stay outside these domains. Share immutable runtime inputs,
not mutable application state or policy authority.

Darling's rootless namespaces and same-UID socket checks do not encode different
application grants. Compare distinct service credentials with an equally
qualified process/LSM boundary. Test cross-domain access and server-mediated
operations, not only direct worker calls. A writable overlay must not shadow
admitted executable providers or turn guest-writable settings into owner policy.

Query actual Linux capabilities and establish controls before guest
initialization. Missing required controls refuse enforced launch. Audit does
not apply optional policy restrictions to obtain observations; missing
observations are reported honestly. Preserve all existing contract distinctions
between admission, supervision, observation and enforcement.

## Setup and activation

First establish the upstream core/system control route. Then compare two
explicit prototypes: rootless Darling with a post-setup restriction/activation
barrier, and an existing outer launcher with Darling's non-root copy mode.
Neither rootless nor non-root mode supplies this complete policy. Reuse existing
Linux service/namespace tooling rather than creating a privileged launcher
before demonstrating it is needed.

Namespace/mount preparation and confinement activation need a declared order.
Landlock restrictions cannot simply be applied before mount setup; they also
do not retract pre-opened descriptors. All helpers and the server must be
covered before untrusted initialization is released. If an upstream activation
hook is required, keep it narrow and test both required denial and ordinary
loader operation. Never silently switch execution modes after an enforced
route fails. [Boundary research](../research/2026-11-25-linux-enforcement.md).

## Allocation

All rows require implementation review and evidence for the selected Linux
image, architecture, backend and execution route. Candidate mechanisms are
alternatives to evaluate, not a claim that one mechanism covers an entire row.

| Control | Owner | Reuse candidate and qualification boundary |
| --- | --- | --- |
| `launch.supervision` | Platform/runtime | Existing Linux process/service management, child-worker barrier and lifecycle handles; prove cleanup, crash containment and descendant reaping |
| `resource.limits` | Platform | cgroup v2 for aggregate memory/process accounting plus supervisor time/output bounds; verify descendants cannot escape the controlled group |
| `filesystem.grants` | Security/platform | Landlock or suitable LSM with scoped filesystem view and trusted object resolution; test aliases, rename, inherited descriptors, mmap and broker access |
| `network.policy` | Security/platform | Network namespace and appropriate Linux access controls for v1 deny/allow-all; test IPv4/IPv6, UDP, existing sockets, descendants and server-mediated access |
| `process.descendants` | Runtime/security | Shared child admission and inherited OS restrictions; test direct fork/exec, native helpers, daemonization and unreviewed executable attempts |
| `process.inspection` | Security/platform | Enforced v1 permits only deny: evaluate Linux credentials, namespace/LSM/ptrace policy; test same-user peers and compatibility-server memory access |
| `memory.executable` | Runtime/security | Enforced v1 permits only deny for runtime-generated executable memory: qualify mappings, executable backing objects, descriptors and aliases while permitting admitted loader transitions; the two-check RWX/RW-to-RX probe was insufficient |
| `ipc.services` | Security/platform | Per-domain endpoints and authenticated brokers; test Unix sockets, Mach compatibility IPC, descriptor transfer and cross-domain access; separate privileged shell protocols from application access |
| `devices.grants` | Security/platform | Scoped device visibility, permissions/LSM and brokered access; test direct device opens/ioctls, inherited handles and graphics requirements |
| `code.dependencies` | Runtime | Reuse loader admission and maintain immutable dependency identity; test initial/transitive/injected/dynamic libraries and raw alternate loading routes |
| `identity.immutable_inputs` | Runtime/security | Content-addressed or sealed/pinned inputs through execution; test path replacement, cache mutation, retained descriptors and grant binding |
| `identity.signature` | Security | Maintained content/CMS/chain verification and explicitly evaluated upstream trust-store code; bind exact executed bytes and owner anchors; CDHash presence alone is insufficient |
| `identity.entitlements` | Runtime/security | Integrity-bound parsing and final policy aggregation; test initial/dynamic code, unknown keys, canonical encodings and conflicting declarations; libDER adoption is held pending failed semantic expectations |
| `launch.environment` | Runtime | Existing process-launch APIs with explicit owner-resolved environment; test injected loader variables, working directory, native/interpreted parity and audit semantics |
| `policy.storage` | Security/platform | Protected Linux storage with atomic updates and recovery; test unauthorized edits, rollback/substitution and restart persistence |
| `broker.authorization` | Security/platform | Kernel-established peer credentials plus launch-scoped grants and lifetime binding; test forged/replayed requests, PID reuse and confused-deputy behavior |
| `audit.events` | Runtime/security | Existing Linux/runtime observation mechanisms and protected event transport; distinguish observed operations from predictions and missing coverage |

Seccomp limits exposed system calls; the Linux documentation explicitly does
not treat it as a complete sandbox. Landlock restricts supported resource
operations and requires feature discovery. Neither automatically supplies the
full owner-policy semantics above. A service-manager hardening option or Docker
container is a candidate mechanism, not a completed control family.

Darling's current sandbox implementation records profile state without
enforcement. Its loader's ptrace setting and server's cross-process memory access
must be included in the threat model. Code-signing modes and local trust stores
are separate reuse candidates; do not rename them as Burdettie's full security
modes without a semantic comparison.

## Executed evidence and its limits

The [local Linux probe](../research/2026-11-25-linux-enforcement.md) installed an
additional seccomp filter and demonstrated one selected raw-syscall denial in
a child and grandchild with a permitted control operation. Namespace-creation
commands succeeded. Landlock, named Unix sockets, Darling memory operations,
cgroup delegation and a user systemd manager were unavailable or blocked in
this environment. These outcomes do not qualify a control family or identify
the capabilities of the future product kernel.

A separate filter denied RWX `mmap` and RW-to-RX `mprotect` but permitted one
memfd to have both writable and executable aliases. No instructions were
written or executed. This counterexample rejects those two checks as a complete
implementation of `memory.executable`; it is not a test of systemd's complete
implementation. Include writable executable backing files, inherited handles,
aliases and dependency admission in the next experiment. Legitimate loader
protection changes also need testing, not a blanket exception for all runtime
code generation.

The [PureDarwin probe](../research/2026-11-25-puredarwin-reuse.md) demonstrated
libDER Linux portability but failed two of 30 semantic expectations. Its C
build and sanitizer observations do not establish signature/entitlement parser
fitness. Resolve those contracts and regressions before adopting the parser
in a trust decision. Verification, owner authorization and confinement remain
separate results.

## Shipping gate

Owner-authorized debugging or JIT experiments belong to a separately scoped
development route or a future reviewed contract version. They are not implicit
exceptions to v1's deny-only inspection/generated-code controls. A backend that
requires such behavior cannot claim that enforced v1 profile without satisfying
the contract or explicitly refusing the combination.

For each advertised control, prove actual denial via imported APIs, raw Linux
operations, inherited handles, descendants and relevant server/broker requests.
Include successful permitted operations and cleanup after rejection/death.
Record kernel configuration and actual capability availability. Do not mark
`available`, `verified` or `established` based on a plan, parsed profile, mock
record or successful compatibility function.

Native ELF, native Mach-O, interpreted Mach-O and embedding are separate routes.
An unrestricted in-process embedding path may remain a development interface;
it must not claim confinement of its own host process. Required-control failure
must occur before application initialization on every enforced route.

## Sources and next work

- [Research RQ-03](../RESEARCH_QUESTIONS.md#rq-03--owner-trust-and-linux-enforcement)
- [Linux Landlock](https://docs.kernel.org/userspace-api/landlock.html)
- [Linux seccomp](https://docs.kernel.org/userspace-api/seccomp_filter.html)
- [Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html)
- [Darling sandbox source](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/sandbox/sandbox.c)
- [Darling loader process access](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/mldr/mldr.c)

Implement and test one declared worker/control set first. Extend the existing
machine-readable allocation/validation only with explicit Linux backend
identity and real evidence; do not mutate the historical PureDarwin records to
make the selected Linux backend appear qualified.
