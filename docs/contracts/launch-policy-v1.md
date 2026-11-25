# Launch and selectable security contract version 1

Status: M1 design contract for implementation in M2 through M6. It describes required behavior, not capabilities already delivered. Source baseline: `7bf375562ca9edf513c353da7ec3309c0337f457`.

## Decision

Every supported application launch enters one trusted supervisor API. The supervisor authenticates the caller, resolves the registered application and owner-selected profile, binds the decision to immutable executable inputs, checks the selected backend, and creates a managed application worker. Native execution, interpretation, command-line execution, the desktop launcher, and supported embedding adapters consume the same authorized launch plan. No adapter constructs a permissive fallback when policy is absent or unsupported.

Security mode controls how configured policy decisions affect execution. Policy contents separately control what is allowed. An enforced profile may intentionally permit unsigned code while denying network access. Permissive operation remains an owner-selectable supported choice. Neither the word `secure` nor the presence of a signature proves confinement or authenticity.

The owner may administer the machine; an application does not inherit administrative authority merely because its owner can use sudo. Enforced workers must be unable to modify their supervisor, policies, grants, trust anchors, executable store, or another application's state.

## Mode semantics

| Mode | Policy evaluation | Policy effect | Reporting |
| --- | --- | --- | --- |
| `permissive` | Optional checks may be skipped | Optional Burdettie policy does not deny or transform guest actions | Skipped checks are `not_evaluated`; never `verified` |
| `audit` | Evaluate the selected policy wherever observation is available | Record `would_deny`, `would_strip`, or `would_limit`; do not apply those optional restrictions | Declare missing observations and audit coverage; incomplete audit is not a clean result |
| `enforced` | Evaluate every required admission control and establish every required runtime control | Deny before guest initialization if any required control denies, cannot decide, or cannot be established | Report actual policy revision, actual backend and scope; no silent downgrade |

All modes still obey host permissions, structural format validation, runtime correctness limits, user cancellation, and implementation resource limits needed to keep the system operational. These have separate reason categories and are not described as policy denials. An unreadable executable, malformed request, unsupported architecture, or unavailable runtime can therefore fail in any mode. Audit must not secretly clear injected libraries or sanitize a delivered environment because its hypothetical enforced policy would have done so. Separate unavoidable platform transformations must be reported.

Version 1 intentionally limits network rules to explicit `deny` or `allow_all`; destination-specific rules require a later version with qualified resolution and connection semantics. Resource limit `null` means no optional policy ceiling; zero is not a synonym. Native execution requires matching host and guest architecture; the interpreter route reports both architectures separately.

Mode is resolved from the owner's global default or per-application selection before the launch request is accepted. Missing mode, unknown mode/version, malformed policy, or unresolved profile identity is a configuration error, not permission to run permissively. An explicit owner-configured permissive default is valid.

## Versioned messages

Use documented JSON wire objects and JSON Schema in M1. M2 implements matching Rust types and shared evaluation/orchestration. This avoids introducing a parallel executable security subsystem before its callers and enforcement backend exist. Schema validity establishes shape only; cross-field invariants, identity authority, and real enforcement require implementation tests.

All messages have integer `schema_version: 1` and a `message_type` discriminator. The schema defines `launch_request`, `policy_snapshot`, `backend_capabilities`, `launch_report`, and `lifecycle_event`. The launch report is the pre-initialization admission record; subsequent state changes are lifecycle events. Policy-affecting unknown fields or enum values are rejected. Extensions require a new supported schema version or an explicitly non-authoritative metadata namespace. The supervisor must not reinterpret newer policy versions as older defaults.

### Launch request

This is a request from an authenticated owner session or an explicitly delegated service, not a grant from guest-controlled input.

| Field | Requirement |
| --- | --- |
| `schema_version` | Exactly `1` |
| `request_id` | Caller correlation token; not authorization or replay protection |
| `app_id` | Opaque registration ID issued by the supervisor |
| `expected_revision` | Expected application content revision; mismatch requires a new inventory and decision |
| `security.mode` | `permissive`, `audit`, or `enforced`; required |
| `security.profile.id` and `security.profile.revision` | Exact owner-controlled policy snapshot |
| `execution_preference` | `auto`, `native`, or `interpreted`; actual route must be returned |
| `arguments` | Explicit argument array; no shell interpolation |
| `environment` | Explicit requested name/value map; no ambient process-environment inheritance |
| `document_grants` | Opaque references to owner-issued grants; paths supplied by an application are not grants |
| `working_directory` | A named app-data or granted-directory reference, not an arbitrary privileged host path |

Local path launch is implemented as an authenticated registration or ephemeral inventory step followed by this request. Inspect-only commands may accept paths without launching; `macho-inspect --exec` must use the shared launch path. The CLI, GUI, and embedding API do not each invent different defaults.

### Application identity

The supervisor creates an identity containing registration ID, content revision, executable digest, selected Mach-O slice and architecture, bundle resource manifest reference, dependency-provider manifest reference, and signature/entitlement evidence. Bundle identifier, display name, path, Team ID, and apparent vendor name are untrusted metadata until independently established; none alone grants rights.

The same verified bytes must be mapped or executed. Pin opened objects, use a verified immutable content store, or another demonstrably equivalent mechanism; never verify a path and later reopen potentially replaced bytes. Resolve dependencies and inserted libraries into their own immutable identities. Dynamic loads repeat this admission process before executing any initializer. Runtime-generated executable memory is a separate policy control and capability, not implicitly authorized by a signature on the main executable. Version 1 restricts this field to `deny`; a future grant needs an exact reviewed memory policy and qualified backend. It does not prohibit ordinary mapping of already admitted executable code.

Persistent grants bind to the registered application and reviewed revision/update rule. A newly substituted executable cannot acquire old grants by copying a bundle identifier. Updates trigger re-inventory and re-evaluation before any grant is carried forward.

### Owner policy snapshot

The policy snapshot includes its schema version, profile ID/revision/digest, authenticated owner or delegated authority, and explicitly configured controls. Required control families are:

- Executable integrity and signer policy: unsigned/ad-hoc acceptance, required page and special-slot integrity, required CMS verification, allowed trust anchors/signers, relevant validity/timestamp/revocation/offline behavior.
- Entitlement requests: version 1 uses `deny_keys`, `request_only_keys`, and the explicit `unknown_keys` behavior. A listed deny key causes denial when present, regardless of its value, and takes precedence over the other lists. Request-only keys grant nothing regardless of value. Other keys are denied or recorded according to `unknown_keys`. This version has no value-predicate DSL; typed predicates require a later contract. Entitlement evidence remains distinct from owner-issued grants.
- Code loading: main executable, complete dependency closure, inserted libraries, dynamic loads, allowed shim/native providers, executable-memory/JIT rules, and environment transformations.
- Resources: read/write/create permissions for identified filesystem objects or trees, network policy, IPC/service access, subprocess policy, process inspection/debugging, executable-memory limits, and resource ceilings requested by that profile.
- Observability: policy decision coverage, audit destination, required event durability, and behavior if a mandatory audit/control channel fails.

Process inspection is restricted to `deny` in version 1. Later support must bind owner grants to exact target identities and operations. Do not use a generic `sandbox: true` field as the complete contract. Controls must identify resources, operations, and backend capabilities needed to enforce them. An explicitly unrestricted family is allowed and visible; omitted fields do not silently mean unrestricted. In a restricted allowlist, unmatched operations are denied in enforced mode. Deny predicates take precedence over matching grants. Do not permit ambiguous first-match rules.

Filesystem grants are resolved against pinned objects or safely anchored trees with explicit symlink, rename, and hard-link behavior. Raw string prefix checks are insufficient. Special devices, executable files, supervisor sockets, and process-control endpoints require distinct grants or remain unavailable. Network defaults to an explicit `deny` or `allow_all`; destination restrictions are accepted only when the backend defines resolution, actual destination, protocol, port, and reconnect semantics. No promise of hostname restrictions is made by comparing a requested hostname string alone.

### Allowlist and reviewed reference semantics

Empty lists never mean unrestricted resource access. A filesystem `allowlist` with no grants permits no optional file operations. Each file grant contributes only its listed operations. IPC `allowlist` with no service references, `allow_reviewed` devices with no grant references, and `allow_reviewed` subprocesses with no reviewed application digests admit no corresponding access. `deny` ignores no hidden grants: its associated grant list must be empty. `unrestricted` IPC/filesystem/subprocess modes must likewise have no contradictory grant list.

Inserted and dynamic libraries marked `allow_reviewed` must match `code_loading.reviewed_dependency_sha256`; an empty list permits no such library. Subprocess `allow_reviewed` uses `resources.subprocess.reviewed_app_sha256`. Device `allow_reviewed` uses supervisor-resolved `resources.devices.grant_refs`. No unspecified reviewed-target registry or caller assertion supplies an implicit grant. Process inspection and runtime-generated executable memory remain `deny` only in this schema version.

For a `trusted_signer` code policy, `trust_anchor_sha256` must be nonempty. Each `allowed_signers` entry is a lowercase SHA-256 digest of the exact DER-encoded leaf signing certificate whose public key actually verified the CMS signature. Display names, Team IDs, subject strings, SPKI hashes, and unrelated certificates merely embedded in the CMS are not accepted as this identifier. An empty `allowed_signers` adds no signer-identity filter beyond successful code-signing validation to the configured anchors; a nonempty list requires the verified signer leaf certificate digest to match. Certificate rotation requires a profile update when an identity allowlist is present. It never bypasses signature, chain, usage, or time validation. Empty reviewed dependency/application lists do not inherit this special signer-filter meaning. `signed_integrity` requires signed content and may permit ad-hoc signatures; `trusted_signer` rejects unsigned and ad-hoc content. `none` makes no executable authenticity claim, even when other resource controls are enforced. Profiles using `none` or `signed_integrity` keep trust-anchor and signer-identity lists empty so unused trust configuration cannot imply verification.

### Backend capability manifest

A capability manifest names backend ID/version, OS/kernel build, host and guest architectures, execution route, worker strategy, and each available control. For each control report:

- Implementation state: `unsupported`, `implemented_unverified`, or `verified`.
- Runtime availability: available, unavailable, or failed to establish.
- Kind and scope: `admission`, `observation`, `resource_enforcement`, or `supervision`; native/interpreter coverage; descendants; supported operations and limits. Each required capability specifies its expected kind and scopes. A supplied capability must match the kind exactly; admission cannot substitute for resource enforcement.
- Evidence reference: test/source revision and platform tuple that justify a verified claim, bound to the exact capability ID, kind and covered scopes. Required capabilities have at least one explicit scope. Evidence for another control, kind or narrower scope cannot qualify the advertised control.

These are separate dimensions. A verified backend feature may be unavailable on this boot; a compiled feature may never have been qualified. Matching a trait method, mock implementation, or `is_enforcing()` boolean is insufficient. The supervisor derives required capabilities from the selected profile, not from a client-provided capability list. Any unsupported, unverified, unavailable, or incorrectly scoped required control prevents a production enforced launch. Development experiments may exercise unverified implementations but cannot emit a successful enforced qualification report.

Every allowed enforced report requires the reserved `launch.supervision` capability of kind `supervision`, qualified and established for both `application` and `descendants`, plus nonempty finalized control findings. This records the managed-worker baseline without calling it a sandbox. Required policy capabilities are additional and are derived from the full owner policy. Deleting the required list cannot produce a valid enforced success report. The document validator checks the supplied records; the actual supervisor must still establish that no required policy control was omitted.

A `resource_enforcement` capability in this contract represents an optional selected-policy restriction. It cannot be established in audit or permissive mode. Managed supervision, host permissions, and disclosed non-policy operational ceilings remain possible; policy controls must not be relabelled as supervision to evade mode semantics. Optional policy cannot be the cause of an audit/permissive refusal. Malformed policy configuration is a configuration error instead.

Admission checks and continuous resource enforcement are separate capabilities. A worker process provides a crash boundary; it provides the advertised security boundary only when actual kernel/backend restrictions also constrain direct syscalls, native FFI, inherited handles, IPC and child processes. An interpreter running in the supervisor with host-pointer access is not isolation.

### Effective launch report

Produce a supervisor-authored report before guest initialization and append authenticated lifecycle/control events afterwards. Include:

- Schema version, launch ID, request correlation, source/image version, application identity and selected slice.
- Requested mode/profile revision and actual effective mode/profile revision; effective mode is null when launch is refused before establishment.
- Selected route, backend/version, capability evidence and runtime establishment results.
- Per-control findings: `allow`, `deny`, `unknown`, `unsupported`, `not_evaluated`, or `not_applicable`; decision reason; observed vs cryptographically verified evidence; intended action and actual action.
- Admission disposition: `allow`, `deny`, or `error`; audit completeness; effective environment key names and transformations; grants/limits actually established.
- Lifecycle state, start/end times, exit code or signal, cancellation/timeout reason, and final resources released.

Separate `finding` from `action`: an audit finding may be `deny` while action is `observed_only`; a permissive skipped verifier is `not_evaluated`, not `allow`. A refused launch has no effective mode or profile. An allowed launch cannot contain an actual `denied` action. `unknown`, `unsupported`, and `not_evaluated` findings cannot claim verified evidence; skipped findings explicitly carry `not_evaluated` evidence. Final admission aggregation is computed after every relevant verdict is finalized. No entitlement or dependency evaluator may mutate a decision after the final aggregate has been computed.

Do not print environment secrets, credentials, document contents, or reusable grant tokens into reports. Record names, policy-relevant transformations, digests where appropriate, and opaque non-reusable identifiers. Reports are readable by the owner and protected from guest edits.

## Required execution sequence

1. Authenticate request authority; validate exact schema, identity/profile revisions, arguments and resource references.
2. Inventory and pin the executable, bundle resources, selected architecture, initial dependencies, and requested environment without executing guest code.
3. Resolve owner policy and backend. Evaluate all admission controls against the final inventory. For enforced, derive and require the exact capabilities needed by that policy.
4. Create the worker and establish resource restrictions, minimal handles, IPC/broker endpoints, limits, and report/event channels. Only explicitly authorized handles cross the boundary. The supervisor retains control of termination and policy storage.
5. Confirm establishment, publish effective report, then permit dependency constructors, Objective-C `+load`, guest initializers and entry. A denied launch produces no guest side effects.
6. Admit dynamic loads and subprocesses before their code executes. Restrict each child at least as much as its parent's effective policy; creating a helper or using host fallback cannot shed restrictions.
7. Track `starting`, `running`, `exited`, `crashed`, `cancelled`, or `timed_out` from actual worker events. Reap the process and release resources. A failed app must not terminate or corrupt the desktop/supervisor.

There is no retry under weaker policy after denial. Route fallback is permitted only when it preserves the same authorized identity, effective policy, required capabilities, and report; otherwise return an actionable error. Policy changes apply at next launch in the alpha. To revoke access from a running app, terminate and relaunch it unless the backend has demonstrated complete live revocation, including already-open handles and children.

Limit units and scopes are explicit: wall-clock milliseconds per launch, memory bytes per process tree, process count, and output bytes per channel. Implement only limits the backend can establish for the claimed scope. An optional policy ceiling is audited rather than applied in audit mode; a distinct runtime-operational ceiling or explicit owner cancellation remains operative and is labelled accordingly.

## Embedding contract

The supported secure embedding API is a supervisor client with versioned request/report objects and a managed-worker handle. It exposes start, wait/status, cancel, and owner-authorized profile operations; language bindings have the same semantics.

Legacy in-process C/Rust embedding remains a documented development/permissive capability until migrated. It cannot claim application confinement while the guest shares the caller's address space and authority. Preserve existing ABI symbols during migration, but do not reinterpret `darwin_runtime_run_secure` as proof of this new enforced contract. A future versioned API can explicitly refuse unsupported enforced operation. Calling the existing C wrapper is not sufficient M2 evidence.

## Implementation allocation

| Area | M2 or later implementation |
| --- | --- |
| Shared contract types and policy aggregation | Shared runtime/security layer; schema version and aggregate semantics owned once |
| Desktop, shell, inspector execution, compatibility runner | Thin adapters; all launch through supervisor, distinguish native demos from Mach-O guests |
| Native and interpreter engines | Consume authorized plan; no per-engine mode defaults or independent authorization logic |
| Platform backend | Worker lifecycle and actual capability negotiation; enforcement implementations qualify in M5 |
| Code-signature verifier | Independent evidence production; authenticity is not inferred from certificate presence |
| Embedding bindings | Versioned supervisor client; legacy ABI remains explicitly scoped |

The current `PlatformProvider` supplies host service abstractions but its process trait exposes only PID/main-thread information. Add a separate process-supervision/confinement interface; do not label ordinary filesystem or memory provider wrappers a sandbox. Security-sensitive runtime helpers receive a launch-bound context rather than looking up mutable process-global policy.

The [M1 Darwin backend allocation](BACKEND_ALLOCATION.md) and its [machine-readable catalog](backends/puredarwin-x86_64.json) assign each control to an owner and a concrete next mechanism or explicit unsupported status. The catalog includes inspected native PureDarwin sandbox hooks and their limitations; it does not promote those source findings into guest enforcement evidence.

## Acceptance cases

1. The same application, mode and profile revision produce equivalent admission findings through CLI, GUI, interpreter, native route, inspector execution, and supported embedding. Unsupported routes refuse instead of silently defaulting.
2. A denied main executable, entitlement, initial dependency or inserted library cannot run a constructor that writes a marker. A denied dynamic library cannot execute its initializer.
3. Audit records the same hypothetical denial and does not block, strip or limit the optional policy action. Missing observation is labelled incomplete. Permissive skipped checks never become verified successes.
4. Required unknown/unsupported signature or resource capability prevents enforced guest initialization. A profile allowing unsigned code can still succeed with real filesystem/network restrictions; signer requirement is independent of mode.
5. Replacing executable/dependency bytes between inventory and execution cannot result in running unverified replacements. Copying an app ID/bundle identifier or report cannot mint grants.
6. Filesystem/network/subprocess controls are demonstrated through both compatibility APIs and direct operations that bypass those APIs. Descendants and inherited handles remain constrained. M5 owns this real backend evidence; schema/logic tests cannot close it.
7. A fake CMS containing a trusted root certificate is not called verified. Tampered code or entitlements fail the requested integrity policy. A valid chain for another purpose does not establish code-signing authorization.
8. App crash, cancellation and timeout leave the supervisor/session alive and release children/handles. Effective state follows worker events rather than an optimistic launcher flag.
9. Changes of mode, profile or grants require owner authority, survive restart, and apply to a newly launched worker. Guest code cannot edit its own effective policy.

## Current source evidence

All links are pinned to the reviewed main revision.

- [`RuntimeConfig`](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/macho-runtime/src/config.rs) currently represents no policy as `None` and secure as `apple_default`.
- [`pipeline_native.rs`](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/macho-runtime/src/pipeline_native.rs) calls shared security evaluation; [`pipeline_interp.rs`](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/macho-runtime/src/pipeline_interp.rs) omits the call and returns no security report.
- [`macho-inspect` execution](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/macho-inspect/src/exec.rs) is a separate in-process route with ambient environment inheritance.
- [`trust-policy` aggregation](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/trust-policy/src/lib.rs) replaces the enriched entitlement verdict after the base aggregate is computed.
- [`CMS verifier`](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/trust-policy/src/cert.rs) ignores its CodeDirectory input and returns `signature_verified: false`; [`sandbox shims`](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/libsystem-shims/src/darwin_stubs/sandbox.rs) explicitly accept profiles without enforcement.
- [`C FFI run API`](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/darwin-runtime-ffi/src/run.rs) offers in-process permissive/secure wrappers, and [`platform process API`](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/platform-core/src/process.rs) has no worker or confinement contract yet.

M1 can approve this contract and its valid/invalid wire examples without claiming M2 integration or M5 enforcement. The platform milestone must name the backend that can implement the required controls before enforced OS support is advertised.

## Contract tooling

See `README.md` beside `launch-v1.schema.json` for wire examples and the record validator. Its tests reject unsafe report shapes and inconsistent claims. They do not authenticate the supervisor, derive the required controls from a policy, verify executable bytes or signatures, resolve grants, establish confinement, or qualify a backend. Those are M2 through M6 implementation and evidence requirements.
