# Linux confinement research: whole-runtime boundaries

Recorded 2026-11-25 (America/Denver). Implements research for RQ-03/RQ-08
under [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md).
This record contains source inspection, narrow executed mechanism probes and
design recommendations. It does **not** qualify a Burdettie enforced launch.

Follow-up: the [retained AWS host probe](2026-11-25-aws-host.md) passed selected
socket, memory, namespace and Landlock prerequisites that this restricted
workspace blocked. Its SSM-inherited process context and capacity stop are
recorded separately; it does not replace the local results below or qualify
the shipping worker service.

## Decisions this evidence supports

1. Evaluate the complete upstream **rootless** Darling route first. Current
   source contains it; a privileged Burdettie launcher is not a prerequisite
   established by this research.
2. Make the confinement unit a complete application trust domain: its
   `darlingserver`, prefix, launchd, workers, descendants and helpers. Start with
   one application/profile per domain. Do not share an owner-wide server across
   applications with different grants.
3. Keep the owner supervisor and authoritative policy/trust state outside that
   unit. Share an immutable runtime image, not mutable application state or
   policy authority. A writable prefix overlay must not silently replace
   admitted executable providers.
4. Reuse Linux service/container mechanisms and small Rust policy/lifecycle
   adapters. Test bubblewrap for a constrained filesystem/namespace view and
   systemd for lifecycle/cgroup management; neither is selected as a complete
   implementation of launch-policy-v1.
5. Keep enforced v1 qualification separate from compatibility progress. This
   environment cannot establish several required controls. Its successful
   seccomp and namespace probes do not justify weakening the contract.

These are architecture recommendations from this work. Adoption and actual
capabilities remain governed by the canonical strategy and recorded backend
qualification.

## Executed host observations

The probe ran as UID 1000 on x86_64, kernel release `6.18.57`, with no effective
or bounding capabilities. Its parent already had `NoNewPrivs: 1`, seccomp mode 2
and one existing filter. Compiler: GCC `14.2.0-19` from Debian.

Reproduce from the repository root:

```sh
python3 scripts/research/linux_controls_probe.py
```

The [runner](../../scripts/research/linux_controls_probe.py) compiles the
[source-owned C fixture](../../scripts/research/linux_controls_probe.c) with
`cc -std=c11 -O2 -Wall -Wextra -Werror`. It prints JSON including source hash,
commands, exit codes and individual syscall results. Temporary files are
removed. Restriction changes affect disposable children only. The probe does
not alter host policy, mount filesystems, run services, contact a network or
execute generated code. Exit 77 means a subprobe was unavailable or blocked.

| Probe | Observed result | What this establishes |
| --- | --- | --- |
| C fixture compile | Exit 0, no warnings/errors | Reproducer builds on this host |
| Additional seccomp filter | Installed in child | Unprivileged filter stacking works here |
| Raw `getppid` after filter | `-1`, `EPERM` in child and grandchild | The selected denial applies to a direct syscall and is inherited |
| Raw `getpid` after filter | Success | The fixture still performs an allowed operation |
| Landlock ABI query | `-1`, `ENOSYS` | Landlock unavailable through this execution environment |
| Landlock ruleset create | `-1`, `ENOSYS` | Filesystem/inheritance tests could not run |
| `process_vm_readv` against fixture's own process | `-1`, `EPERM` | A memory operation used by Darling cannot be demonstrated here |
| `PR_SET_PTRACER_ANY` on fixture | `-1`, `EINVAL` | The loader's ptracer setup cannot be demonstrated here |
| Fixture child reading fixture parent memory | `-1`, `EPERM` | No positive cross-process-memory control result |
| AF_UNIX stream socket | Socket creation fails with `EPERM` | No prefix stream listener can be demonstrated |
| AF_UNIX datagram socket | Socket creation fails with `EPERM` | Darling's RPC listener cannot be demonstrated |
| AF_UNIX stream `socketpair` | Success, fixed byte transferred | Paired local descriptors work; this does not qualify named listeners |
| User namespace and single-ID mapping | Exit 0 | Creation/map operation accepted |
| User + mount, UTS, IPC, network or PID namespace | Each creation-only command exited 0 | Creation accepted; no isolation/security claim |
| cgroup controller/delegation files | Not visible | No aggregate resource-control qualification |
| User systemd manager | `offline`, exit 1 | Transient-unit qualification unavailable here |
| bubblewrap | Version `0.12.0` available | Tool present; a sandbox configuration was not qualified |

`ENOSYS`/`EPERM`/`EINVAL` do not identify whether the cause is kernel
configuration, an emulated syscall surface or an enclosing sandbox/filter.
Do not infer Landlock availability from the kernel version string. Repeat on
the actual product/test host with its declared configuration; retain blocked
results rather than manufacturing a fallback success.
In particular, socket creation and memory-operation failures prevent a complete
Darling control run here even though namespace creation is accepted.

### Executable-memory result

A separate child installed a deliberately small W^X-style filter. It denied
an RWX `mmap` and an RW-to-RX `mprotect`, both with `EPERM`. The same child then
successfully created a `memfd` and mapped it through **both a writable alias
and an executable alias**. No instructions were written or executed.

This is a negative result for the hypothesis that those two restrictions alone
implement v1's runtime-generated-executable-memory denial. It is not a test of
systemd's complete implementation. The
[systemd documentation](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html#MemoryDenyWriteExecute=)
also describes writable executable filesystems and `memfd_create` as concerns.
Code admission, writable storage, descriptors and aliases must be addressed
together. Ordinary mapping of already admitted code must remain supported.

## Darling source changes the deployment choice

Inspected pins:

- Darling `90af44da21d314b9acd24de1d1272e4aba8b9cc6`.
- darlingserver `3604875f2d3fa46985e914650d23ce124f99ac27`.

The current [launcher](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/darling.c)
selects root, rootless or non-root execution. Rootless uses a user namespace
mapping the invoking user to namespace UID 0, and creates mount/UTS namespaces.
Unprivileged installs may resolve their image relative to the launcher.
The [server mode contract](https://github.com/darlinghq/darlingserver/blob/3604875f2d3fa46985e914650d23ce124f99ac27/internal-include/darlingserver/run-mode.hpp)
distinguishes rootless from non-root copy mode; the latter has no private PID
namespace. Upstream's [rootless tests](https://github.com/darlinghq/darlingserver/blob/3604875f2d3fa46985e914650d23ce124f99ac27/tests/rootless.md)
explicitly distinguish host-only mode tests from complete startup qualification.

This supports using upstream's rootless lifecycle before inventing a privileged
service. It does not make rootless a security policy: current startup retains
host SysV IPC for X11 interoperability, and its prefix exposes the host through
`/Volumes/SystemRoot`. A new user namespace still maps to the invoking host
user's file authority. Our complete enforced route must constrain that authority.

### Why the server and prefix belong to the trust domain

The [server](https://github.com/darlinghq/darlingserver/blob/3604875f2d3fa46985e914650d23ce124f99ac27/src/server.cpp)
has a per-prefix Unix socket with mode 0600 and `SO_PASSCRED`.
[Call admission](https://github.com/darlinghq/darlingserver/blob/3604875f2d3fa46985e914650d23ce124f99ac27/src/call.cpp)
checks the kernel-supplied UID against the server UID. Those checks distinguish
Linux users; they do not encode Burdettie's application/profile grants for two
applications running as one user. Per-prefix state is useful but needs an
external access boundary.

The [memory implementation](https://github.com/darlinghq/darlingserver/blob/3604875f2d3fa46985e914650d23ce124f99ac27/src/process.cpp)
uses `process_vm_readv`/`process_vm_writev`. The
[loader](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/mldr/mldr.c)
sets `PR_SET_PTRACER_ANY` because its server is outside the worker PID namespace.
That relaxes Yama's relationship restriction; it does not override other Linux
permission/LSM checks. [Yama documentation](https://docs.kernel.org/admin-guide/LSM/Yama.html).

**Design inference:** put all authority serving one application inside one
outer constrained domain. Share read-only runtime inputs across domains, and
use separate prefixes, server sockets and lifecycle/accounting. Compare a
distinct service credential per domain with an equally qualified LSM/process
boundary; do not assume a UID mapping alone supplies isolation. A distinct
credential needs a working Darling account/home lookup and storage lifecycle,
so it is a mechanism candidate rather than a tested requirement.

Server-internal copying needed to implement an authorized runtime operation is
different from granting an application arbitrary inspection. Both raw Linux
inspection calls and guest-facing Mach task/inspection requests must be tested.
The server's task-port hook currently allows requests when its signing policy
is off. Burdettie's inspection policy therefore cannot be inferred from
upstream signing mode. Likewise, guest UID 0 authority over upstream system
configuration is not equivalent to our authenticated owner authority.

## Setup, activation and storage need an explicit boundary

Darling's namespace/overlay setup happens before the application runs.
Landlock filesystem restrictions prevent subsequent mount/topology changes;
apply them at a qualified point after necessary trusted setup and before
untrusted initialization. They must cover the server and its future children,
not only a final application thread. Landlock domain nesting also constrains
ptrace relationships. Its file-open restrictions do not retract pre-opened
descriptors. [Landlock semantics](https://docs.kernel.org/userspace-api/landlock.html).

**Design inference:** the supervisor should prepare a narrow outer filesystem
view and admitted runtime inputs, then start the complete rootless runtime.
Qualification must establish when remaining restrictions become effective,
which handles survive and when application initialization is released. If a
small upstream activation hook is necessary, contribute that hook; do not
replace the runtime to avoid defining the boundary.

Compare two explicit enforcement prototypes after the upstream control works:

| Prototype | Question it resolves |
| --- | --- |
| Rootless Darling with a post-setup restriction/activation barrier | Can required restrictions cover the server and workers after namespace assembly but before application initialization? |
| Existing outer launcher plus Darling's non-root copy mode | Can the outer launcher provide the filesystem/PID/network boundary, allowing the complete upstream runtime to avoid inner mount setup? |

The second route inherits whatever isolation the outer launcher establishes;
Darling's non-root mode does not supply that isolation itself. Its lifecycle,
server memory access, data paths and cost need their own records. Select a
route before launch and report it truthfully. Neither route is a silent
fallback from failed rootless or enforced startup.

A read-only lower image plus an unrestricted writable overlay is insufficient
for immutable code identity: the upper layer may shadow library/config files.
Allow writes to declared application data/state, keep authoritative policy
outside guest-writable storage, and test dynamic/injected dependencies against
the admitted provider manifest. No guest-writable upstream preference or trust
store automatically becomes the owner policy source.

Blanket executable-memory hardening also needs loader testing. Darling's
[image protection implementation](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/mldr/image_protection.c)
contains an anonymous writable copy followed by `mprotect`, remapping and
sealing for admitted images. A filter can deny a legitimate loader transition
as well as a JIT transition. Preserve v1's deny-only inspection/generated-code
semantics; an incompatible route reports unsupported. A later contract change
would require an explicit design, not a silent runtime exception.

## Reuse plan and next qualification

| Component | Reuse role to evaluate | Burdettie work |
| --- | --- | --- |
| Upstream rootless Darling | Whole compatibility runtime and lifecycle | Rust request/report adapter; per-domain configuration |
| bubblewrap | Filesystem/namespace construction | Derive explicit mounts, handles and endpoints from owner policy |
| systemd and cgroup v2 | Service lifecycle and aggregate process-tree limits | Map launch IDs to supervised units; report established limits |
| Linux LSM/Landlock and seccomp | Resource restrictions and syscall surface | Capability discovery, activation order and exact negative tests |
| Existing protocol proxies/brokers | Narrow desktop/service access | Authenticate grants and preserve per-application identity |

Bubblewrap explicitly leaves policy definition to its caller and warns that
exposed service sockets can convey authority. Reuse an existing protocol proxy
where suitable rather than passing the owner session bus wholesale.
[bubblewrap policy scope](https://github.com/containers/bubblewrap/blob/main/README.md).
Seccomp filters syscall operations rather than interpreting safe path objects;
it is one mechanism, not a complete sandbox.
[seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html).
Aggregate limits need an actual controlled cgroup and descendant placement.
[cgroup v2 documentation](https://docs.kernel.org/admin-guide/cgroup-v2.html).

The next useful execution host is an existing Linux machine/VM exposing the
required mechanisms and Darling's memory operations. Run a source-owned
application in one domain, then a second domain with different grants. Require
permitted operations to succeed, denied raw/library/broker operations to fail,
cross-domain access to fail, and cancellation to reap every helper. Test alias
paths, inherited descriptors, server requests and executable-memory aliases.
Keep the prototype's optional policy restrictions out of audit/permissive mode;
do not label restrictions as operational limits to evade their mode semantics.

No new paid infrastructure, host security configuration change, upstream
submission, or production capability claim was made by this research.
