# M1 development without Mac hardware

Current source selection: PureDarwin `main` at `0c1990c803c13b68cad4df3f6dc34ba4306fc789`. The November 14 source preflight and 20.5 image boot remain historical for their exact older inputs; current-main build/boot is not yet qualified. See [the source refresh record](M1_PUREDARWIN_SOURCE_REFRESH_20261123.md).

Use a Linux build host and the pinned x86_64 Darwin VM as the first Burdettie reference platform. Mac hardware is not required for boot, persistence, source-owned Mach-O fixtures, shared launch/policy integration, or testing resource controls inside that VM. The production goal remains a Darwin-compatible OS running real macOS applications.

## Actual blockers

The retained AWS Free plan instance is the intended execution host. The November 14 record documents QEMU/OVMF, Nix 2.28.3 and the complete checkout of the superseded `0163b2d` source. The refreshed `main` checkout and its changed corecrypto submodule have not yet been verified on the host. The original bootstrap result has been retrieved: it failed after 600.166 seconds because the harness missed the rendered prompt. The interrupted follow-up is retained. The corrected `bootstrap-20261114-02` verified a fresh Darwin/x86_64 shell nonce and exact file bytes across two QEMU processes, in 235.948 and 233.682 seconds. This is a measured success for the older 20.5 image only. Guest `sync` followed by host termination and ext4 journal replay does not establish graceful shutdown. See the [bootstrap evidence](evidence/bootstrap-2026-11-14.json) and [AWS development checkpoint](M1_AWS_DEVELOPMENT.md).

A historical no-build plan for the superseded `0163b2d` source's `apple-sdk`, `native-ld`, `darwin-cross-toolchain` and `libsystem` passed in 232.21 seconds. This is not a current-main plan; rerun evaluation against the refreshed source before reusing any output. It lists 32 local derivations and 264 fetched paths: 916.73 MiB planned download and 4811.47 MiB planned unpacked data. There were 14387019776 free disk bytes at that checkpoint; retain an 8-GiB reserve. Individual no-build plans have also passed; the SDK-only plan is 3 local derivations and 1908.90 MiB unpacked, and the plans overlap. The bounded SDK-only realization then passed in 75.11 seconds, with required headers/link stub checked and 12243050496 disk bytes still free. Neither those estimates nor upstream's full-image 60-GB guideline determines the narrow fixture build's peak usage. The [source-preflight evidence](evidence/source-preflight-2026-11-14.json) records those earlier planning and SDK results.

The later [native-linker attempt](evidence/native-linker-2026-11-14.json) has now been retrieved and diagnosed. It failed after 1201.12 seconds when its phase deadline stopped libtapi at 26%, compiling `CodeGenDAGPatterns.cpp`. The full log contains no compiler-fatal diagnostic, and the cgroup recorded no memory-max or OOM event. Its failed tree and logs remain preserved. At that historical recovery checkpoint, 9554071552 free bytes left only 964136960 bytes (about 0.90 GiB) above the reserve; the same driver's 3-GiB growth-budget preflight would have rejected another attempt. New staging requires separate review, derivation/output identities, offline planning and measured capacity; another Nix invocation does not resume that retained tree. The later staged success below leaves libSystem realization, executable linking, selected image build/boot and runtime execution open.

The later [opt-in libtapi staging metadata plan](evidence/libtapi-plan-2026-11-14.json) passed in 24.08 seconds, listing only its new candidate derivation and no dependency fetch paths. The source remains complete; its 217,415,680 allocated bytes are compared with a 50,094,080-byte staging-only estimate. This metadata plan ran no candidate unpack, compilation, realization, native-linker rebinding or guest execution. The metadata phase ended with 9,525,194,752 free bytes, still above the unchanged 8-GiB reserve. This plan does not qualify build peak capacity or close an execution gate.

A separate [staged libtapi build](evidence/libtapi-build-2026-11-14.json) subsequently passed in **4,082.6 seconds**, finishing at 20:08:20 UTC. Its complete captured log and output inspection establish the installed x86_64 ELF library, eight regular headers and exact NAR identity. Minimum observed free space was 8,995,893,248 bytes, above the 8-GiB reserve, with no cgroup high/max/OOM event. The original failed tree remains retained. The following [native-linker metadata plan](evidence/staged-linker-plan-2026-11-14.json) passed in **32.21 seconds**, with **nine local derivations and zero fetch paths**, and freshly rechecked complete-source and installed-output identities. It finished with 9,289,572,352 free bytes.

The native build then passed in **459.99 seconds** and its installed-output inspection in **14.29 seconds**, finishing at **21:08:52 UTC**. Its [complete 32-file evidence archive](evidence/staged-native-linker-recovered-2026-11-14.json) is recovered and independently reviewed: exact registration, native file hashes, static ELF and observed staged-TAPI loader binding, full logs and sampled resources agree. Minimum observed free space was **9,175,244,800 bytes**, with **203,493,376 bytes** maximum observed whole-host net growth and no high/max/OOM event. The direct cgroup peak was **553,099,264 bytes**; the journal's separate **478.5M** summary is preserved without equating them. The earlier [terminal observation](evidence/staged-native-linker-terminal-observation-2026-11-14.json) remains an unchanged historical record of the interrupted transfer. Fresh current-output/source checks remain prerequisites to downstream planning; this native stage does not qualify any later build peak or guest execution.

The Linux/PureDarwin fixture recipe and C ABI probe are implemented. Both fixture sources compiled successfully to x86_64 Mach-O objects with the selected SDK and its already-realized LLVM 21.1.8 compiler. The offline stage took 12 seconds for preflight and 11 seconds for compilation/inspection, with no new dependency download. Executable linking and execution remain unrun; see the [object evidence](evidence/objects-2026-11-14.json). The reviewed [backend allocation](../contracts/BACKEND_ALLOCATION.md) covers all 17 control families, all unavailable/unsupported without guest qualification evidence. The feature branch now includes [PR #7](https://github.com/burdettadam/burdettie-os/pull/7) through `c6b40679da2a814008bc5f6ec030e784c369d0ff`, preserving admission, launch-owned cleanup, maintained fonts/alignment and the typed NSRange adapter, and adding the [bounded printf/owned-string slice](https://github.com/burdettadam/burdettie-os/blob/c6b40679da2a814008bc5f6ec030e784c369d0ff/docs/INTERPRETED_PRINTF.md). The original ARM64 NSCharacterSet fixture passes its semantic exit check and matches all 415 expected stdout bytes on the Linux x86_64 interpreter host; see the [execution receipt](evidence/interpreted-printf-2026-11-14.json). Native x86_64 Darwin linking, selected-guest execution and enforcement remain unqualified.

The earlier [runtime/platform receipt](evidence/runtime-platform-integration-2026-11-14.json) retains `aade848`'s seven passing full-CI jobs and failed repository hygiene; the [prior integration receipt](evidence/runtime-integration-2026-11-14.json) retains its two failed gates. The additive [final `c6b4067` receipt](evidence/interpreted-printf-ci-2026-11-14.json) records all seven full-CI jobs passing and both required repository-hygiene runs failing on two history findings. Combined-feature-head checks remain separate, and main remains unchanged.

The earlier requirement to identify and boot a physical Mac before closing M1 was a sequencing choice. M1 now freezes the virtual hardware and tests it first. Physical driver/firmware qualification is deferred. The exact Stickies bundle is an M4 application input, not a prerequisite for writing the loader, supervisor or security implementation.

## Primary execution path

1. Recover and independently review the already-captured native terminal archive, then freshly verify its output. Reuse the existing x86_64 Linux host and retained source, disks and logs. Measure the next narrow dependency stage against fresh free space and the 8-GiB reserve before realization, preserving the shared host's CPU/memory limits and services. If it cannot fit, record the measured blocker and revise the bounded plan; do not expand resources, add infrastructure or introduce a paid upgrade.
2. Once its prerequisites fit, build the pinned minimal image and run the offline QEMU recipe in [ADR 0001](../adr/0001-darwin-boot-substrate.md). The provisional profile is two vCPUs and 2048 MiB, using single-thread TCG under the existing host limits. It is derived from upstream and unqualified on the selected image; the upstream four-vCPU/4096-MiB profile was not executed. TCG does not require KVM or nested hardware virtualization. Record tool/image hashes and exact VM configuration.
3. Prove shell access with a unique guest-command response, write a known file, stop QEMU completely, restart the same writable disk and verify the exact bytes. A kernel banner or an echoed command alone is insufficient.
4. Use the implemented [Linux/PureDarwin recipe and ABI probe](M1_FIXTURE_ABI.md) to cross-build `simple_cli` and `m1_abi_probe` for x86_64 with the pinned SDK, compiler, linker and libSystem. First run them directly in the selected Darwin guest; then execute them through Burdettie's runtime inside that guest. Record each route, output and exit/cleanup result separately. Continue with Objective-C/Foundation and the document fixture only after the narrow ABI path is understood.
5. Implement and qualify the supervisor and selected policy controls on this virtual platform. VM isolation protects a host from a guest; it does not by itself isolate one application from other processes inside Burdettie. File, network, process and descendant restrictions still require the M5 tests.

PureDarwin constructs its SDK from source headers and supplies its cross toolchain on Linux. Burdettie's [new fixture builder](../../scripts/build_puredarwin_fixtures.py) is separate from the existing Darwin/`xcrun` ARM64 script. It verifies the exact revision, lock and submodule, resolves the four already-realized outputs without starting dependency builds, and refuses missing inputs. It follows `nix/lib/target-info.nix` at the pinned revision: the deployment target is macOS 26.5, and overriding the wrapper target can change the output architecture. This label does not imply complete macOS 26.5 API support. The fixture's allocation, file, thread and exit checks pass on the Linux host; selected-SDK object compilation has passed; executable linking and selected-guest execution remain unqualified. See the [fixture gate](M1_FIXTURE_ABI.md), pinned [target selection](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/lib/target-info.nix) and [source SDK recipe](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/apple-sdk-pinned.nix).

## Completed older-image bootstrap

The official prebuilt PureDarwin 20.5 minimal image supplied the bounded Linux/QEMU bootstrap. Its completed result establishes the tested shell/serial automation and exact-file readback before a selected-source build.

| Input | Exact value |
| --- | --- |
| Release | [PureDarwin 20.5](https://github.com/PureDarwin/PureDarwin/releases/tag/20.5) |
| Asset | `puredarwin-minimal.img` |
| Asset bytes | 274726912 |
| SHA-256 reported by GitHub | `ef65a816395680ef9210f401cebe3783c131badca1a942cb80e56bec97e717ac` |
| Release source commit | `1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290` |

The downloaded image was verified against that digest and run through private writable copies with the recorded Ubuntu QEMU/OVMF variant. This older release uses a different substrate from both the superseded `0163b2d` candidate and the refreshed current-main candidate `0c1990c803c13b68cad4df3f6dc34ba4306fc789`. Its pass cannot qualify the current candidate, Burdettie's runtime, Apple application compatibility or security enforcement. Preserve the original failed run and all retained disks/logs; any necessary follow-up gets a new output directory. See the [AWS checkpoint](M1_AWS_DEVELOPMENT.md), [sanitized evidence](evidence/bootstrap-2026-11-14.json) and [release runner source](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/nix/image-contents.nix).

Continue on the existing AWS Free plan host. Ordinary repository checks remain required, but no new CI execution host, paid runner or larger artifact/storage budget is part of this platform experiment.

### Bounded bootstrap harness

[`scripts/m1_vm_bootstrap.py`](../../scripts/m1_vm_bootstrap.py) accepts an already-downloaded official image and a matching OVMF CODE/VARS pair. It verifies the pinned image size and SHA-256, creates private writable copies, runs an offline headless TCG VM, and records `blocked`, `fail` or `pass` with raw serial logs and hashes. A pass requires a nonce-verified Darwin/x86_64 shell response and exact-byte persistence across two separate QEMU processes. Echoed commands and a kernel banner are insufficient. The first process is stopped after guest `sync`; this does not test graceful guest shutdown.

```sh
python3 scripts/m1_vm_bootstrap.py \
  --image /path/to/puredarwin-minimal.img \
  --firmware-code /usr/share/OVMF/OVMF_CODE_4M.fd \
  --firmware-vars /usr/share/OVMF/OVMF_VARS_4M.fd \
  --output-dir /path/to/new-run \
  --memory-mib 2048 --vcpus 2 --timeout 600
```

This is the existing bootstrap recipe, not a request to repeat the completed run. Confirm the firmware paths before any necessary follow-up; its output directory must be new and its parent must exist. On the retained shared host, keep the unprivileged cgroup's 3-GiB memory ceiling, no swap and 75%-of-one-CPU quota. The script additionally bounds CPU affinity, wall time, RSS and logs. Its two-vCPU/2048-MiB profile now also bounds the provisional selected-source VM configuration; that candidate still requires a separate run. The harness never downloads software, installs packages, changes its input files or opens public listeners. Unit tests validate protocol and failure handling; actual execution supplies boot evidence.

## What can be proved without a Mac

| Work | Evidence available on the reference VM |
| --- | --- |
| Darwin image boot, shutdown, persistence and recovery | Independent OS operation on the declared virtual hardware |
| Source-built Mach-O and Cocoa fixtures | Specific loader, ABI and framework behavior |
| Supervisor and security profiles | Actual admission decisions and guest OS restrictions, when exercised through direct operations and descendants |
| An existing macOS app bundle | Static metadata on Linux/Windows, then actual workflow behavior under Burdettie once dependencies are supported |
| Physical Mac drivers, firmware, battery and sleep | Deferred; VM results cannot establish those device-specific claims |

An exact Apple-authored application remains a separate acceptance target. Source fixtures are useful controlled inputs and do not substitute for running Stickies or another named Apple-shipped app. If a legitimate existing bundle is available, static inventory already works without a Mac:

```sh
python3 scripts/collect_m1_inventory.py --no-hardware --app /path/to/Stickies.app > app-inventory.json
```

Native macOS comparisons can later use recorded reference observations or a reference environment. They are valuable for undocumented behavior, but not a universal prerequisite for the defined fixture/boot/security tasks.

## Revised milestone gates

- **M1:** Virtual reference platform, source fixture/build contract, shared launch/security contract, recorded VM build/boot/runtime feasibility and explicit backend gaps.
- **M3:** Bootable VM preview with session, persistent data and recovery.
- **M4:** The exact real macOS application completes its useful workflow; Apple-authored claims require that actual app.
- **M5:** Real enforcement inside the selected Darwin VM, including operations that bypass compatibility shims.
- **M7:** Installable and recoverable VM alpha. Physical-machine support is separately qualified when selected and available.

M1 remains in progress. Bootstrap readback, SDK/object compilation, staged libtapi and native-linker realization with recovered inspection evidence, fixture-recipe implementation and backend allocation review are complete within their stated scopes. The remaining gates are fresh current-output verification, downstream realization within the existing host, fixture cross-build, the selected image's own boot/persistence/recovery results, and Rust/runtime ABI execution in that guest. The [staged downstream path](M1_STAGED_LIBRARIES.md) preserves explicit verification before each next stage. All 17 backend control families remain unavailable/unsupported, with zero qualified. No physical Mac is required for these VM milestones.
