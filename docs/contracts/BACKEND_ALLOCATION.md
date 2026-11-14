# M1 Darwin backend allocation

The [machine-readable allocation](backends/puredarwin-x86_64.json) assigns all 17
control families to engineering owners, inspected source, candidate mechanisms,
remaining gaps and required qualification tests. It covers the selected x86_64
PureDarwin guest. **Every control remains unsupported for a production Burdettie
enforced launch on that guest.** No built image or runtime control has been
qualified by this allocation.

The inspected Burdettie source is
[`4e804df245b122137d67c45f9d026e261ac68e42`](https://github.com/burdettadam/burdettie-os/tree/4e804df245b122137d67c45f9d026e261ac68e42).
PureDarwin is pinned at
[`0163b2db225b85e3bf96fc25646ea290105ac298`](https://github.com/PureDarwin/PureDarwin/tree/0163b2db225b85e3bf96fc25646ea290105ac298).
The native managed-worker context in the allocation is the intended integration
target. Its `not-built` image and `not-observed` kernel fields explicitly prevent
it from being read as an observed runtime environment. The allocation is
engineering data, not an authorization document to accept from an application.

## Source findings that affect the platform decision

There is an open-source enforcement candidate beyond Burdettie's compatibility
stubs. The pinned PureDarwin
[`libsandbox`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libsandbox/sandbox.c)
compiles its own native profiles and invokes a MAC policy syscall. The pinned
[`Sandbox.kext`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Kernel/Extensions/Sandbox/sandbox_kext.c)
registers vnode operations, socket bind/connect checks and credential-label
inheritance. This gives the platform team a specific implementation to evaluate
before considering a new policy engine.

Three limitations are visible in that source and must shape the experiments:

- Apple's compiled profile format is accepted as a permissive marker. The
  `sb_apple_check` compatibility path answers operation queries permissively.
  Neither an accepted Apple profile nor a successful `sandbox_check` result
  establishes confinement. Qualification must exercise actual operations under a
  native PureDarwin profile and observe the denial.
- Native profile decisions use paths and the last matching rule. Burdettie's
  contract instead requires owner-resolved object/tree grants and deny
  precedence. Translation, alias/rename behavior and retained handles require
  explicit implementation and negative tests.
- The inspected socket hook table covers bind/connect. It does not establish
  full deny-all coverage for UDP `sendto`, existing connected sockets, inherited
  handles, every address family or descendants. Similar direct-operation tests
  are required for file mappings, process inspection and executable memory.

The pinned
[`nix.conf`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/nix/nix.conf)
disables Nix's Seatbelt sandbox. That setting does not erase the native policy
implementation above or qualify it. Source availability and successful guest
enforcement are separate evidence states.

Burdettie's own
[`sandbox` compatibility shims](../../crates/libsystem-shims/src/darwin_stubs/sandbox.rs)
return success without enforcing a policy. Its
[`ProcessInfo`](../../crates/platform-core/src/process.rs) trait supplies identity
queries, while the [Darwin filesystem provider](../../crates/platform-darwin-host/src/fs.rs)
delegates to ordinary filesystem operations. These are compatibility services;
they do not implement the supervisor or resource boundary described here.

The [current CMS routine](../../crates/trust-policy/src/cert.rs) ignores the
CodeDirectory input and returns `signature_verified: false`. Finding an Apple
root fingerprint among embedded certificates is insufficient to establish a
signature or valid signing chain. Signature presence, parsed entitlements and
compatibility results must remain distinct from authenticated evidence.

## Allocation map

All rows have `implementation: unsupported`, `availability: unavailable` and no
qualification evidence on the selected Burdettie guest. The JSON contains exact
source paths and test cases for each row.

| Control | Owner | Candidate and outstanding boundary |
| --- | --- | --- |
| `launch.supervision` | Platform lead | Separate supervisor, worker barrier, lifecycle/reaping; reuse guest launchd to start the supervisor |
| `resource.limits` | Platform lead | Supervisor time/output limits; aggregate guest process-tree memory/count mechanism unresolved |
| `filesystem.grants` | Security/platform | Native PureDarwin vnode policy; object grants, aliases, retained handles and deny precedence unqualified |
| `network.policy` | Security/platform | Native socket policy candidate; complete direct-operation and descendant coverage unqualified |
| `process.descendants` | Security/runtime | Reviewed child admission plus inherited restrictions; direct fork/exec and daemonization unqualified |
| `process.inspection` | Security/platform | Guest task/debug access checks; per-app denial not implemented |
| `memory.executable` | Security/runtime | Kernel denial of generated executable memory while permitting admitted code; mechanism unqualified |
| `ipc.services` | Security/platform | Restricted service namespaces and authenticated brokers; direct IPC and handle transfer unqualified |
| `devices.grants` | Security/platform | Owner-issued grants plus device/IOKit boundary; broker and kernel coverage missing |
| `code.dependencies` | Runtime lead | Shared admission for initial/transitive/inserted/dynamic libraries and providers; full integration missing |
| `identity.immutable_inputs` | Runtime/security | Pinned objects or immutable content store; substitution/cache/grant binding unqualified |
| `identity.signature` | Security lead | Maintained content/CMS/chain verifier; exact executed-byte binding and trust behavior missing |
| `identity.entitlements` | Security/runtime | Integrity-bound entitlement evaluation with final aggregation; guest integration unqualified |
| `launch.environment` | Runtime lead | Owner-resolved mode/profile and explicit environment; adapter parity and audit behavior incomplete |
| `policy.storage` | Security/platform | Protected durable profile/grant store; authenticated edits, atomic writes and recovery missing |
| `broker.authorization` | Security/platform | Kernel-established peer identity and launch-scoped grants; trusted endpoint missing |
| `audit.events` | Runtime/security | Protected launch/lifecycle/control events; complete direct-operation observation missing |

[PR #5](https://github.com/burdettadam/burdettie-os/pull/5), inspected at
`4a74a51502a29bc65ef1fe4a8ce92ce2c11ec498`, already advances native/interpreted
admission parity and final entitlement aggregation. It was published and
unmerged at this allocation checkpoint. Preserve and review that work; this
change does not reimplement it. Its code tests do not qualify the new guest or
provide resource confinement.

## Mode and evidence rules

Permissive mode skips optional controls without inventing successful evidence.
Audit evaluates and reports would-deny outcomes where observation exists, while
leaving optional restrictions unapplied. The native sandbox must not be applied
merely to obtain audit logs. Missing observations remain explicit. An enforced
launch requires every control derived from its complete profile to be verified,
available and established before any guest initializer runs.

Allowing unsigned code is a separate owner policy decision. It does not remove
filesystem, network or other required controls from an enforced profile. The
Linux cgroup and network namespace surrounding an experiment's QEMU process
protect the build host; they do not qualify any of these guest controls.

Qualification evidence in the wire schema now binds the capability ID, kind and
covered scopes as well as the exact backend/route/platform context. This closes
a document-validation gap where an unrelated supervision result could be reused
to label a resource control verified. Required capabilities must also name at
least one scope. These checks detect inconsistent records; they do not prove
that an evidence reference is genuine or that a runtime actually applied it.

## Validation and promotion

From the repository root, with the pinned contract development dependency:

```sh
python3 docs/contracts/validate_contract.py --self-test
python3 docs/contracts/validate_contract.py docs/contracts/examples/*.json \
  --allocation docs/contracts/backends/puredarwin-x86_64.json
```

The self-tests include complete allocation coverage, missing owners/tests,
scope/kind weakening, host-versus-guest confusion, unobserved image promotion,
unpinned/private path references and the evidence-binding regressions. The
existing M1 contract workflow runs them through `--self-test`.

To promote a capability, implement the allocated mechanism, run its negative and
positive cases in the exact guest, retain reviewed sanitized evidence tied to the
image digest and source revisions, and update only the scopes actually proved.
A source inspection, a schema test, a kernel banner, an accepted profile or a
successful compatibility call cannot provide that promotion. M1 platform and ABI
feasibility and M5 enforcement qualification remain distinct gates.
