# M1 platform and security contracts

Status: **in progress**. The design and investigation targets below are ready for developer review. Closing M1 still requires the actual Mac and application inventory and a recorded boot feasibility result. This milestone defines the first bootable Darwin-compatible Burdettie OS, with owner-selectable permissive, audit and enforced security.

Source baseline: [`7bf375562ca9edf513c353da7ec3309c0337f457`](https://github.com/burdettadam/burdettie-os/commit/7bf375562ca9edf513c353da7ec3309c0337f457), which integrates recovery PR #1. The M1 changes define contracts and add investigation tools; they do not change runtime admission or claim a working security boundary.

## Decisions developers can implement

| Decision | Contract |
| --- | --- |
| Product | Bootable Darwin-compatible OS that completes named macOS application workflows |
| First substrate experiment | Pinned PureDarwin/XNU on x86_64; production adoption follows the bounded feasibility result |
| Reproducible VM | QEMU q35, TCG IvyBridge, four vCPUs, 4096 MiB, OVMF X64, AHCI disk and serial console; initially offline |
| Physical machine | One available older Intel Mac, selected from its actual inventory; model and device qualification remain open |
| Application candidate | Locally supplied Apple Stickies, using an inventoried x86_64 slice; exact version and dependency closure must qualify |
| Public fixtures | Existing `fixtures/simple_cli/main.c` for first execution; `BurdettieDocument` 0.1.0 for the document workflow specified below |
| Security modes | `permissive`, `audit`, `enforced`, independent of signer requirements and resource grants |
| Launch ownership | One authenticated supervisor owns identities, profiles, grants, effective reports and worker lifecycle |
| Application boundary | Enforced workers have no authority over supervisor state; real backend restrictions are required beyond creating a process |
| Development lanes | Native macOS ARM64 and Linux ARM64 interpreter tests continue; they do not qualify the x86_64 OS lane |

Read [ADR 0001](../adr/0001-darwin-boot-substrate.md) for the substrate choice, pinned build/boot recipe and stop conditions. Read the [version 1 launch and policy contract](../contracts/launch-policy-v1.md) for message fields, mode semantics, capability negotiation and implementation allocation. The [target manifest](m1-targets.json) records unresolved fields as null rather than invented values.

## Platform scope

The build host and the shipping guest are different roles. Linux is the selected PureDarwin build environment and an existing runtime validation backend. The candidate guest uses Darwin/XNU. A full macOS restore-image VM remains a reference environment. Neither a Linux desktop demo nor the macOS reference VM closes independent OS boot acceptance.

The OS experiment uses x86_64 application slices. ARM64 interpretation on x86_64 remains an investigation path, not an assumed substitute for x86_64 ABI qualification. If the available Mac or application requires another architecture, revise the target matrix explicitly before broadening implementation.

The initial alpha is bounded to one physical machine, one repeatable VM, one Apple application and one public document fixture. It requires boot, display and input, persistent app/user data, recoverable session configuration, shared launch/profile controls, and the declared application workflow. Wi-Fi, audio, GPU acceleration, suspend, battery management and extra displays are qualified individually only if required by the selected machine/use case. iOS/UIKit, App Store/iCloud integration, full Xcode, arbitrary macOS applications and additional hardware families are outside this alpha.

## Application target and workflow

**Stickies is the first candidate, pending the local bundle inventory.** Record its bundle identifier, marketing/build versions, source macOS version/build, selected Mach-O slice, main executable and bundle-resource digests, minimum OS/SDK commands, direct and transitive dependency graph, and required services. Do not call it a public-framework-only target until that graph and the exercised workflow establish the claim.

The useful task is to create a note, enter and edit representative Unicode text, exercise selection and clipboard, quit, cold reboot and reopen the exact note contents. Repeat the workflow on three consecutive documented cold boots. An open window or a successful process exit alone is insufficient. If the exact Stickies version needs unsupported private dependencies, record the blocker and make a deliberate target/version decision; do not replace the app silently with a Rust demo.

`BurdettieDocument` 0.1.0 is the public fixture specification for M4. Build a source-owned Cocoa/Mach-O app with an x86_64 slice and a minimal programmatic AppKit interface, avoiding an initial dependency on nib compilation or `NSDocument`. Use a text view, an explicit file-open action, save action and visible error state. The fixture must open a known UTF-8 file, edit and save exact bytes, quit/reboot/reopen them, and show useful errors for missing or unwritable files. Keep its sources and build recipe public; build products remain generated artifacts. This fixture is specified here and is not implemented or reported as passing by M1.

The same document fixture later exercises an enforced file denial, a readable explanation, an owner grant and a successful relaunch. Stickies retains its own natural notes-persistence workflow. Both use the same launch/profile contract.

## Inventory commands

On the intended Mac, from a checkout containing this change, run:

```sh
python3 scripts/collect_m1_inventory.py --host-role physical-target --app /System/Applications/Stickies.app > burdettie-m1-inventory.json
```

Use the actual local application path if it differs. The collector reads allowlisted hardware and bundle metadata without launching the application. It does not upload files. Its hashes and signing metadata identify inputs; they do not establish code-signature authenticity, a complete bundle integrity manifest, a transitive dependency graph or compatibility. Complete the remaining fields through the follow-up tasks below. Review the generated inventory before sharing it; keep the application and local observations out of the public source repository.

If the app comes from a different Mac, use `--host-role source-oracle` there and collect the intended physical target separately. The collector's current macOS version does not establish the original source of a previously copied bundle. See [M1 inventory details](M1_INVENTORY.md) for the coverage and remaining provenance fields.

On the x86_64 Linux build host, after obtaining the pinned PureDarwin checkout described in ADR 0001:

```sh
python3 scripts/m1_boot_preflight.py --source /path/to/puredarwin-m1
```

The preflight never builds, downloads or boots an image. Exit 2 means prerequisites are blocked; exit 0 means ready to attempt the spike. Neither outcome establishes boot success. The current execution environment lacks Nix and a pinned source/image, so the recorded experiment is blocked at this stage. The VM, runtime and physical-hardware checks remain unexecuted.

## Work allocation and completion evidence

| Work item | Responsible role | Output and acceptance |
| --- | --- | --- |
| M1.1 Hardware identity | Platform lead and machine owner | Inventory selects one available Mac; model, CPU, firmware, RAM, storage, GPU/input and network devices are recorded without private identifiers |
| M1.2 Apple application identity | Compatibility lead | Exact Stickies version/slice/content identity and complete dependency/service map; Mac reference workflow recorded |
| M1.3 Boot feasibility | Platform lead | Pinned image build, VM cold boot and persistence/recovery result, narrow runtime ABI result and physical boot feasibility evidence; blocked/fail/pass kept separate |
| M1.4 Security and launch contract | Runtime and security leads | Versioned schema/examples reviewed; mode, identity, grant, report and capability semantics accepted; each required control assigned to an implementation owner |
| M1.5 Backend capability allocation | Platform and security leads | Exact backend mechanism for each control, or explicit unsupported status; no signature/sandbox success inferred from a stub |
| M1.6 Alpha target freeze | Technical lead | Target manifest completed, candidate accepted or revised based on the experiment, bounded alpha checklist and estimates updated from evidence |

M1.1, M1.2 and M1.4 can proceed concurrently. The platform experiment starts immediately when an equipped build host is available. The shared contract can guide M2 implementation while target evidence is gathered; M1 is not closed by documentation alone.

## Alpha acceptance contract

- The selected image boots independently on the VM and named physical machine and preserves an exact test file across a full shutdown/restart.
- Burdettie's runtime executes the source-built x86_64 probe inside that guest; unresolved host library/display assumptions are recorded and assigned.
- Stickies completes the agreed task and reopens its exact data; the public document fixture proves explicit document I/O and error handling.
- CLI, GUI, interpreter, native execution and supported embedding use the same authenticated launch contract, or clearly refuse unsupported combinations.
- Permissive remains usable; audit records hypothetical policy outcomes without applying those restrictions; enforced never silently downgrades or starts guest initialization without required controls.
- Each advertised control has actual backend evidence on the relevant route, platform and architecture, including direct operations and child processes. A JSON schema test or clean boot does not supply this evidence.
- Mode/profile/grant settings survive restart. App crashes and denials leave the session usable, and broken configuration is recoverable without losing user data.

M3 through M7 deliver this integrated alpha. M1 closes when its targets, contracts and feasibility decisions are sufficiently concrete for those milestones, with measured blockers and engineering estimates attached.

## Required decision before closing M1

The only owner input currently missing is the actual target Mac and the local Apple application inventory. The platform team also needs a host capable of running the pinned Nix/QEMU experiment. Collect the inventory and record the spike result before marking this milestone complete. Production sandbox support, an exact supported Stickies version and an installable OS are not yet established.
