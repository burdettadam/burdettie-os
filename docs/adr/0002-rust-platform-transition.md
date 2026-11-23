# ADR 0002: PureDarwin bridge to a Rust platform

Status: **accepted strategy**, 2026-11-23. Implementation is incremental and remains subject to the qualification gates below.

## Decision

Use current PureDarwin as Burdettie OS's short-term boot and integration platform while progressively replacing it with Burdettie's own Rust implementation. The long-term destination includes both the kernel foundation and core userland required by the declared release target. Running Burdettie's Rust runtime on an unchanged PureDarwin/XNU kernel is an intermediate stage.

The product goal remains a bootable Darwin-compatible OS that completes named, versioned macOS application workflows with owner-selectable permissive, audit and enforced security. Compatibility is defined by supported external behavior and demonstrated workflows. It does not require duplicating every upstream internal design or claiming universal macOS compatibility.

The user has established the short-term PureDarwin approach and the eventual Rust replacement. The sequence below is the engineering plan. Kernel architecture, complete subsystem order, delivery dates and additional hardware targets remain open decisions.

[ADR 0001](0001-darwin-boot-substrate.md) continues to govern the M1 PureDarwin experiment. This ADR supersedes any interpretation of that decision as a permanent commitment to PureDarwin/XNU; it does not replace the current milestone acceptance contracts.

## Near-term baseline

Continue M1 platform/toolchain, image, runtime and security qualification and M2 admission work. The [2026-11-23 source refresh](../milestones/M1_PUREDARWIN_SOURCE_REFRESH_20261123.md) selects PureDarwin `main` at `0c1990c803c13b68cad4df3f6dc34ba4306fc789`. Current-source build, boot, guest runtime and enforcement remain unqualified. Results from the earlier source and bootstrap image remain historical evidence.

“Current PureDarwin” means refreshing from upstream through a reviewed, recorded revision. Each qualification build and test run records the exact commit, lock file, submodule identities and relevant artifact hashes. A source refresh triggers re-evaluation of affected evidence; it does not silently inherit an older image's qualification.

Before further realization, re-evaluate the current source, independently verify any reusable toolchain artifacts against their derivation/output identities, and assess capacity. Continue within the existing resource and shared-service constraints. This strategy decision adds no infrastructure or resource expansion.

## Boundaries to preserve now

### Compatibility and platform services

Keep supported application behavior separate from its implementation. Record the executable formats, architecture, ABI, process/thread semantics, memory mappings, IPC, filesystem behavior and runtime/framework interfaces required by each named target. Include negative cases, lifetime rules and explicitly unsupported behavior.

Use narrow platform interfaces so PureDarwin integration and future Rust implementations can serve the same runtime contracts. Reuse the existing loader, runtime, framework work and fixtures where the contracts apply. Review the [host dependency register](../HOST_DEPENDENCY_REGISTER.md) against current code before treating it as complete. Host libc, Rust standard-library services, Objective-C bridges, unwinding, display/input and direct operating-system calls need explicit porting decisions; existing Rust code is not automatically ready for a new kernel.

Public compatibility and process boundaries must have declared formats and ABI rules. Do not expose incidental Rust layouts or an internal trait's implementation details as a stable foreign ABI. Prefer Rust for new owned components, with narrowly reviewed assembly, unsafe code and FFI where hardware or compatibility requires them.

### Security

Preserve the [launch and policy contract](../contracts/launch-policy-v1.md) across platform replacements. Platform selection and security mode are independent choices. The supervisor continues to own launch identity, grants, policy decisions, reporting and worker lifecycle.

Each implementation must report its actual capabilities. Audit records policy decisions without applying policy restrictions. An enforced launch must fail before application initialization when required controls are unavailable. Before claiming enforced support, each backend must demonstrate real denied operations, inherited restrictions and process boundaries for the execution routes covered by that claim; admission checks or compatibility stubs alone do not establish confinement. A Rust implementation also requires qualification of its unsafe, FFI, driver and application-execution boundaries.

### Evidence and ownership

Maintain a versioned fixture and behavior corpus that identifies the backend, architecture, source and artifact under test. Keep host CI, selected-guest boot/runtime, named-application compatibility and security enforcement as separate evidence categories. PureDarwin is an integration reference with its own gaps; use declared contracts and available native macOS observations to resolve compatibility questions rather than copying an upstream limitation into the product contract.

Record each component as Burdettie-owned implementation, third-party dependency, integration patch or temporary adapter. Preserve notices and provenance. “Our own Rust implementation” describes ownership of the platform architecture and necessary kernel/core services; it does not require rewriting every compiler, bootloader or reusable library.

## Transition stages

The R track is separate from existing M1–M7 milestone numbering.

| Stage | Deliverable | Gate |
| --- | --- | --- |
| R1 — Reference platform | Qualified PureDarwin build/image plus Burdettie runtime and supervisor integration. | Observe the current milestone's build, boot, persistence, runtime and security results on the selected guest. |
| R2 — Substitution boundaries | Reviewed dependency inventory, stable behavior contracts, backend identity and representative fixtures. | Tests expose incompatible substitutions and identify remaining host dependencies. |
| R3 — Rust userland replacements | Bounded replacements for selected libraries or services, introduced one component at a time. | Satisfy the component replacement gate below and retain a documented recovery path. |
| R4 — Rust kernel foundation | Independently booting virtual-machine prototype with interrupt/timer handling, memory management, scheduling, processes, IPC, storage and the drivers needed by its declared scope. | Boot and execute a minimal supported program; demonstrate applicable isolation, persistence and recovery behavior. |
| R5 — Integrated Rust platform | Burdettie runtime, supervisor, session and selected application workflows on the new kernel/core services. | Requalify compatibility and all supported security modes on that exact platform. |
| R6 — Independent release target | Reproducible Burdettie image whose required build and runtime path no longer depends on PureDarwin/XNU components. | Meet the declared release target's boot, application, security, persistence and recovery gates independently. |

Start with the existing x86_64 virtual-machine lane to keep the first platform comparison bounded. Additional architectures and physical devices require separate qualification. A kernel architecture decision must explain how its mechanisms support the declared Darwin behavior; this ADR does not select a monolithic, microkernel or hybrid design.

R2 work may accompany M1/M2, and a narrowly scoped R4 prototype may proceed in parallel when resources permit. Neither becomes a prerequisite for finishing the current PureDarwin work. Select the first R3 component after a current dependency audit, using a small observable workflow and a manageable dependency boundary.

## Gate for each replacement

Before a replacement becomes the default for a declared target, record:

1. **Scope and dependencies:** exact interface, architecture, backend, target versions, ownership and remaining upstream dependencies.
2. **Behavior:** positive and negative contract tests, resource/lifetime handling and the representative workflow on both the reference and replacement implementation. Explain intentional differences.
3. **Security:** mode semantics, capability reporting and actual restrictions for the component's authority. Re-run affected backend tests; do not transfer enforcement claims from another kernel.
4. **Persistence and recovery:** relevant on-disk state, migration behavior and a documented rollback or recovery path. Protect existing user data.
5. **Resources and provenance:** measured resource/performance behavior within the available budget, exact source/build/artifact identities and a dependency record showing what was removed.

A component can be complete within a narrow declared scope while the OS transition remains incomplete. The [historical Rust replacement inventory](../BURDETTIE_REPLACEMENT_TARGETS.md) supplies candidates for review, not blanket evidence that these gates already pass.

## Completion and immediate follow-through

Retire PureDarwin as the default only for a target whose independently built Rust-based image boots, runs the declared application workflows, persists data across restart, recovers as specified and passes its security contract on the actual implementation. Audit the required build inputs and runtime image to establish that PureDarwin/XNU components are no longer required. Retain a frozen PureDarwin reference for comparison and historical evidence without making it a dependency of that independent release.

The immediate platform task remains the refreshed-source preflight, toolchain verification and capacity assessment before realization, followed by the existing linking, boot and runtime gates. The immediate Rust transition task is to refresh the dependency inventory and propose one bounded replacement with an explicit fixture and exit gate. This decision records direction; it does not claim a new kernel, completed port, successful guest boot or qualified enforcement.
