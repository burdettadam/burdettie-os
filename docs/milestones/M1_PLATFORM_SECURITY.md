# M1 platform and security contracts

Status: **in progress**. The initial reference platform is an x86_64 Darwin VM to be built and tested from Linux. Closing M1 requires the pinned VM build/boot/runtime feasibility result and the launch/security contracts; physical Mac access and an exact Stickies bundle are deferred qualification work. This milestone defines the first bootable Darwin-compatible Burdettie OS, with owner-selectable permissive, audit and enforced security. See [development without Mac hardware](M1_WITHOUT_MAC.md).

Source baseline: [`7bf375562ca9edf513c353da7ec3309c0337f457`](https://github.com/burdettadam/burdettie-os/commit/7bf375562ca9edf513c353da7ec3309c0337f457), which integrates recovery PR #1. The M1 changes define contracts, implement the fixture recipe and probe, and record platform evidence. The feature branch includes [PR #5](https://github.com/burdettadam/burdettie-os/pull/5) admission, headless/exit prerequisites, maintained fonts/CFConstantString alignment and the typed NSRange adapter through [PR #7](https://github.com/burdettadam/burdettie-os/pull/7).

The current runtime source is `c6b40679da2a814008bc5f6ec030e784c369d0ff`. Its [bounded printf adapter](https://github.com/burdettadam/burdettie-os/blob/c6b40679da2a814008bc5f6ec030e784c369d0ff/docs/INTERPRETED_PRINTF.md) handles the declared Darwin ARM64 stack formats on Unix x86_64 interpreter hosts, with owned immutable string exports. The [exact-source execution receipt](evidence/interpreted-printf-2026-11-14.json) records the original NSCharacterSet fixture's zero exit and exact 415-byte stdout, without a missing-input skip. Its [final component CI receipt](evidence/interpreted-printf-ci-2026-11-14.json) records all seven full-CI jobs passing and both required repository-hygiene runs failing on two history findings. Exact combined-feature-head checks remain required, and main remains unchanged.

On the retained host, the separate [staged libtapi build](evidence/libtapi-build-2026-11-14.json) passed in 4,082.6 seconds and its installed-output inspection passed. The subsequent [staged-native-linker metadata plan](evidence/staged-linker-plan-2026-11-14.json) passed in 32.21 seconds, listing nine local derivations and no fetched paths. The native build then passed in 459.99 seconds and its installed-output inspection in 14.29 seconds. Its [complete 32-file evidence archive](evidence/staged-native-linker-recovered-2026-11-14.json) is now recovered and independently reviewed, including registration, file hashes, ELF/TAPI loader binding, complete logs and sampled resources. The [downstream planner](M1_STAGED_LIBRARIES.md) must freshly verify the current source and outputs before further planning. This successful host-tool stage does not close downstream realization, fixture linking or selected-image boot.

The earlier [tested `aade848` checkpoint](https://github.com/burdettadam/burdettie-os/tree/aade8484a773bfa88433526a2039084cc31d6b20) retains all seven full-CI jobs passing and repository hygiene failing in its [historical receipt](evidence/runtime-platform-integration-2026-11-14.json). The [preceding receipt](evidence/runtime-integration-2026-11-14.json) retains earlier security/hygiene failures. Component or local host results do not establish selected-guest runtime execution or a working security boundary.

## Decisions developers can implement

| Decision | Contract |
| --- | --- |
| Product | Bootable Darwin-compatible OS that completes named macOS application workflows |
| First substrate experiment | Pinned PureDarwin/XNU on x86_64; production adoption follows the bounded feasibility result |
| Selected VM profile | QEMU q35, single-thread TCG IvyBridge, two vCPUs, 2048 MiB, OVMF X64, AHCI disk and serial console; offline and bounded to the existing host. This derived profile remains unqualified for the selected source image |
| Physical machine | Deferred hardware-support track; no physical Mac is required to close the VM-based M1 |
| Application candidate | Stickies remains an M4 candidate; its exact x86_64 version and dependency closure must qualify before an application-support claim |
| Public fixtures | `simple_cli` and `m1_abi_probe` sources and Linux/PureDarwin recipes implemented; selected-SDK x86_64 object compilation passed, executable linking and execution pending. `BurdettieDocument` 0.1.0 remains the M4 document-workflow specification |
| Security modes | `permissive`, `audit`, `enforced`, independent of signer requirements and resource grants |
| Launch ownership | One authenticated supervisor owns identities, profiles, grants, effective reports and worker lifecycle |
| Application boundary | Enforced workers have no authority over supervisor state; real backend restrictions are required beyond creating a process |
| Development lanes | Linux x86_64 build/VM work and Linux ARM64 interpreter validation; existing native macOS tests remain optional reference evidence for this milestone |

Read [ADR 0001](../adr/0001-darwin-boot-substrate.md) for the substrate choice, pinned build/boot recipe and stop conditions. Read the [version 1 launch and policy contract](../contracts/launch-policy-v1.md) for message fields, mode semantics and capability negotiation. The reviewed [backend allocation](../contracts/BACKEND_ALLOCATION.md) assigns all 17 control families to owners, candidate mechanisms and qualification tests; every family remains unavailable/unsupported on the selected guest. Enforced launch must refuse missing required capabilities before guest initialization. The [target manifest](m1-targets.json) records unresolved fields as null rather than invented values.

## Platform scope

The build host and the shipping guest are different roles. Linux is the selected PureDarwin build environment and an existing runtime validation backend. The candidate guest uses Darwin/XNU. A full macOS restore-image VM remains a reference environment. Neither a Linux desktop demo nor the macOS reference VM closes independent OS boot acceptance.

The OS experiment uses x86_64 application slices. ARM64 interpretation on x86_64 remains an investigation path, not an assumed substitute for x86_64 ABI qualification. A future application or physical target that needs another architecture receives a separate target decision and qualification.

The initial alpha is bounded to one repeatable VM, one Apple application and one public document fixture. It requires boot, display and input, persistent app/user data, recoverable session configuration, shared launch/profile controls, and the declared application workflow. Physical-machine support is a later target expansion: firmware, storage, GPU, input, networking, suspend and battery behavior must be qualified on each advertised machine. iOS/UIKit, App Store/iCloud integration, full Xcode, arbitrary macOS applications and additional hardware families are outside this alpha.

## Application target and workflow

**Stickies remains the first Apple-authored GUI candidate for M4, pending the bundle inventory.** Its absence does not block M1 boot, fixture, supervisor or policy work. Before qualifying the app, record its bundle identifier, marketing/build versions, source macOS version/build, selected Mach-O slice, main executable and bundle-resource digests, minimum OS/SDK commands, direct and transitive dependency graph, and required services. Do not call it a public-framework-only target until that graph and the exercised workflow establish the claim.

The useful task is to create a note, enter and edit representative Unicode text, exercise selection and clipboard, quit, cold reboot and reopen the exact note contents. Repeat the workflow on three consecutive documented cold boots. An open window or a successful process exit alone is insufficient. If the exact Stickies version needs unsupported private dependencies, record the blocker and make a deliberate target/version decision; do not replace the app silently with a Rust demo.

`BurdettieDocument` 0.1.0 is the public fixture specification for M4. Build a source-owned Cocoa/Mach-O app with an x86_64 slice and a minimal programmatic AppKit interface, avoiding an initial dependency on nib compilation or `NSDocument`. Use a text view, an explicit file-open action, save action and visible error state. The fixture must open a known UTF-8 file, edit and save exact bytes, quit/reboot/reopen them, and show useful errors for missing or unwritable files. Keep its sources and build recipe public; build products remain generated artifacts. This fixture is specified here and is not implemented or reported as passing by M1.

The same document fixture later exercises an enforced file denial, a readable explanation, an owner grant and a successful relaunch. Stickies retains its own natural notes-persistence workflow. Both use the same launch/profile contract.

## Inventory commands

When an existing application bundle is available on Linux or Windows, inspect it without a Mac:

```sh
python3 scripts/collect_m1_inventory.py --no-hardware --app /path/to/Stickies.app > burdettie-m1-app-inventory.json
```

Physical Mac inventory is optional until a hardware target is selected. On that later target, run:

```sh
python3 scripts/collect_m1_inventory.py --host-role physical-target --app /System/Applications/Stickies.app > burdettie-m1-inventory.json
```

Use the actual local application path if it differs. The collector reads allowlisted hardware and bundle metadata without launching the application. It does not upload files. Its hashes and signing metadata identify inputs; they do not establish code-signature authenticity, a complete bundle integrity manifest, a transitive dependency graph or compatibility. Complete the remaining fields through the follow-up tasks below. Review the generated inventory before sharing it; keep the application and local observations out of the public source repository.

If the app comes from a different Mac, use `--host-role source-oracle` there and collect the intended physical target separately. The collector's current macOS version does not establish the original source of a previously copied bundle. See [M1 inventory details](M1_INVENTORY.md) for the coverage and remaining provenance fields.

On the x86_64 Linux build host, after obtaining the pinned PureDarwin checkout described in ADR 0001:

```sh
python3 scripts/m1_boot_preflight.py --source /path/to/puredarwin-m1
```

The preflight never builds, downloads or boots an image. Exit 2 means its checked prerequisites are blocked; exit 0 means those prerequisites are present, not that a build or boot has passed. Check disk capacity separately.

The retained AWS host now has the exact complete source checkout and submodule, Nix 2.28.3, QEMU/OVMF and the verified older 20.5 image. With the lock unchanged, a no-build plan for `apple-sdk`, `native-ld`, `darwin-cross-toolchain` and `libsystem` passed in 232.21 seconds. It lists 32 local derivations and 264 fetched paths, with 916.73 MiB planned download and 4811.47 MiB planned unpacked data. The checkpoint had 14387019776 free disk bytes and an 8-GiB reserve. Individual no-build plans also passed. The SDK-only stage then built successfully in 75.11 seconds, with required headers/link stub checked and 12243050496 bytes still free. See the [source-preflight evidence](evidence/source-preflight-2026-11-14.json) and [AWS development checkpoint](M1_AWS_DEVELOPMENT.md).

The later [bounded native-linker attempt](evidence/native-linker-2026-11-14.json) failed after 1201.12 seconds at its phase deadline, while libtapi was at 26%. The complete log and receipt are retrieved and verified; no compiler-fatal diagnostic or cgroup OOM event was observed. The failed tree remains preserved. The 18:01 UTC recovery checkpoint left about 0.90 GiB above the 8-GiB reserve, so the same driver's 3-GiB growth-budget preflight would have rejected another attempt. The subsequently reviewed staging recipe used new identities and separately measured capacity; its [successful libtapi build](evidence/libtapi-build-2026-11-14.json) and later native-linker success do not resume the retained failed tree or authorize resource expansion. Downstream stages remain unqualified.

A subsequent [libtapi source-staging metadata plan](evidence/libtapi-plan-2026-11-14.json) passed in 24.08 seconds at `f8c5917`. It lists only the new candidate derivation, with no dependency download or realization, while retaining the complete source and unchanged SDK. Its 50,094,080-byte staging estimate excludes generated/build/output data and does not qualify peak capacity. At the plan's 18:46 UTC finish, 9,525,194,752 free bytes left 935,260,160 bytes above the unchanged 8-GiB reserve. That historical receipt remains `planned_not_built`; the later [build/output inspection](evidence/libtapi-build-2026-11-14.json) and [native-linker rebind plan](evidence/staged-linker-plan-2026-11-14.json) have separate passing receipts.

The [retrieved bootstrap evidence](evidence/bootstrap-2026-11-14.json) preserves the original 600.166-second prompt-matcher failure and the interrupted follow-up. The corrected `bootstrap-20261114-02` passed nonce-verified shell execution and exact-byte comparison across two QEMU processes, with write/read phases of 235.948 and 233.682 seconds. The read boot replayed the ext4 journal after host termination following guest `sync`; this is not graceful guest shutdown. The result qualifies only that older-image bootstrap scope. Selected-image build/boot, linked fixture execution and Burdettie runtime execution remain unrun.

The [fixture and ABI gate](M1_FIXTURE_ABI.md) provides the implemented Linux recipe and source-owned allocation/file/thread/exit probe. Both source-owned probes now compile to thin x86_64 Mach-O objects with the selected SDK and LLVM 21.1.8; the [object receipt](evidence/objects-2026-11-14.json) records the actual results. Object compilation does not establish executable linking or runtime ABI behavior. PR #7 addresses identified host dependencies and exit routing, but Darwin linking, launch-owned normal cleanup and execution inside the selected guest remain unqualified; see its [exit lifecycle design at the published revision](https://github.com/burdettadam/burdettie-os/blob/d8656362b9729ee99f1ae8fe494af81b59ad4637/docs/RUNTIME_EXIT_LIFECYCLE.md).

## Work allocation and completion evidence

| Work item | Responsible role | Output and acceptance | Current state |
| --- | --- | --- | --- |
| M1.1 Virtual platform identity | Platform lead | Pin q35/TCG CPU, firmware, disk, input, image and toolchain identities; choose an equipped Linux build host | Existing host selected; bounded profile defined; selected image/build identities pending |
| M1.2 Public fixture identity | Compatibility lead | Cross-build the source-owned x86_64 probes with exact SDK/linker inputs and expected output; retain real-app inventory as an M4 gate | Selected-SDK x86_64 objects compiled and inspected; executable linking and guest execution pending |
| M1.3 Boot feasibility | Platform lead | Pinned image build, VM cold boot and persistence/recovery result and narrow runtime ABI result; blocked/fail/pass kept separate | Older 20.5 bootstrap passed its narrow scope; selected candidate not built or booted |
| M1.4 Security and launch contract | Runtime and security leads | Versioned schema/examples reviewed; mode, identity, grant, report and capability semantics accepted; each required control assigned to an implementation owner | Contract/evidence validation reviewed; runtime integration remains separate |
| M1.5 Backend capability allocation | Platform and security leads | Exact backend mechanism for each control, or explicit unsupported status; no signature/sandbox success inferred from a stub | Reviewed allocation of 17 families; all unavailable with no qualification evidence |
| M1.6 Alpha target freeze | Technical lead | Target manifest completed, candidate accepted or revised based on the experiment, bounded alpha checklist and estimates updated from evidence | Open pending selected-target feasibility results |

The shared contract can guide M2 implementation while the platform and fixture lanes gather selected-target evidence. Completed recipes, allocation review and host tests are implementation progress; they do not close M1's build/boot/runtime gates.

## Alpha acceptance contract

- The selected image boots independently on the reference VM and preserves an exact test file across a full shutdown/restart. Future physical-machine claims require separate device-specific evidence.
- Burdettie's runtime executes the source-built x86_64 probe inside that guest; unresolved host library/display assumptions are recorded and assigned.
- Stickies completes the agreed task and reopens its exact data; the public document fixture proves explicit document I/O and error handling.
- CLI, GUI, interpreter, native execution and supported embedding use the same authenticated launch contract, or clearly refuse unsupported combinations.
- Permissive remains usable; audit records hypothetical policy outcomes without applying those restrictions; enforced never silently downgrades or starts guest initialization without required controls.
- Each advertised control has actual backend evidence on the relevant route, platform and architecture, including direct operations and child processes. A JSON schema test or clean boot does not supply this evidence.
- Mode/profile/grant settings survive restart. App crashes and denials leave the session usable, and broken configuration is recoverable without losing user data.

M3 through M7 deliver this integrated alpha. M1 closes when its targets, contracts and feasibility decisions are sufficiently concrete for those milestones, with measured blockers and engineering estimates attached.

## Remaining work before closing M1

1. Freshly verify the recovered native-linker output and current source before downstream planning. Measure and realize each remaining cross-toolchain/compiler-rt/libSystem input within the existing host's disk reserve, CPU, memory and time bounds. The completed native stage does not qualify later peak capacity. Stop with a measured blocker if a stage cannot fit; no paid upgrade, new infrastructure or resource expansion is part of this plan.
2. Link the public fixture executables, build the selected image, and record its own independent boot, exact-byte persistence, recovery and graceful-shutdown assessment.
3. Qualify Rust standard-library/libSystem linkage and the headless native x86_64 runtime in that guest, including truthful exit/cleanup results. Host tests and the older image's shell cannot substitute for this execution.
4. Accept or revise the selected target using those results, assign the remaining runtime/session/backend gaps, and attach measured implementation estimates.

Mac ownership and Stickies availability are not M1 blockers. M4 still needs an exact real application and its behavior evidence; physical hardware support needs its own later qualification. Production sandbox support and an installable OS are not yet established.
