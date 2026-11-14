# M1 development without Mac hardware

Use a Linux build host and the pinned x86_64 Darwin VM as the first Burdettie reference platform. Mac hardware is not required for boot, persistence, source-owned Mach-O fixtures, shared launch/policy integration, or testing resource controls inside that VM. The production goal remains a Darwin-compatible OS running real macOS applications.

## Actual blockers

The retained AWS Free plan instance is the execution host. QEMU/OVMF, Nix 2.28.3, the exact complete selected source checkout and its submodule are present. The original bootstrap result has been retrieved: it failed after 600.166 seconds because the harness missed the rendered prompt. The interrupted follow-up is retained. The corrected `bootstrap-20261114-02` verified a fresh Darwin/x86_64 shell nonce and exact file bytes across two QEMU processes, in 235.948 and 233.682 seconds. This is a measured success for the older 20.5 image only. Guest `sync` followed by host termination and ext4 journal replay does not establish graceful shutdown. See the [bootstrap evidence](evidence/bootstrap-2026-11-14.json) and [AWS development checkpoint](M1_AWS_DEVELOPMENT.md).

The selected source's unchanged-lock, no-build plan for `apple-sdk`, `native-ld`, `darwin-cross-toolchain` and `libsystem` passed in 232.21 seconds. It lists 32 local derivations and 264 fetched paths: 916.73 MiB planned download and 4811.47 MiB planned unpacked data. There were 14387019776 free disk bytes at that checkpoint; retain an 8-GiB reserve. Individual no-build plans have also passed; the SDK-only plan is 3 local derivations and 1908.90 MiB unpacked, and the plans overlap. The bounded SDK-only realization then passed in 75.11 seconds, with required headers/link stub checked and 12243050496 disk bytes still free. Larger linker/libSystem stages were not launched. Neither those estimates nor upstream's full-image 60-GB guideline determines the narrow fixture build's peak usage. Remaining linker/libSystem realization, executable linking, selected image build/boot and runtime execution remain open. The [source-preflight evidence](evidence/source-preflight-2026-11-14.json) records the bounded planning result.

The Linux/PureDarwin fixture recipe and C ABI probe are implemented. Both fixture sources compiled successfully to x86_64 Mach-O objects with the selected SDK and its already-realized LLVM 21.1.8 compiler. The offline stage took 12 seconds for preflight and 11 seconds for compilation/inspection, with no new dependency download. Executable linking and execution remain unrun; see the [object evidence](evidence/objects-2026-11-14.json). The reviewed [backend allocation](../contracts/BACKEND_ALLOCATION.md) covers all 17 control families, all unavailable/unsupported without guest qualification evidence. Draft [PR #7's published source](https://github.com/burdettadam/burdettie-os/tree/f6918f029c65144eea3ca7e3b437a58bdefebb4e) contains headless and exit prerequisites; [PR #5](https://github.com/burdettadam/burdettie-os/pull/5) contains the separate, unmerged admission work. Those changes and host tests do not establish Darwin linking, guest runtime execution or enforcement.

The earlier requirement to identify and boot a physical Mac before closing M1 was a sequencing choice. M1 now freezes the virtual hardware and tests it first. Physical driver/firmware qualification is deferred. The exact Stickies bundle is an M4 application input, not a prerequisite for writing the loader, supervisor or security implementation.

## Primary execution path

1. Reuse the existing x86_64 Linux host and retained source, disks and logs. Measure the next narrow dependency stage against its free space and 8-GiB reserve before realization, preserving the shared host's CPU/memory limits and services. If it cannot fit, record the measured blocker and revise the bounded plan; do not expand resources, add infrastructure or introduce a paid upgrade.
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

The downloaded image was verified against that digest and run through private writable copies with the recorded Ubuntu QEMU/OVMF variant. This older release uses a different substrate from the selected `0163b2db225b85e3bf96fc25646ea290105ac298` candidate. Its pass cannot qualify the current candidate, Burdettie's runtime, Apple application compatibility or security enforcement. Preserve the original failed run and all retained disks/logs; any necessary follow-up gets a new output directory. See the [AWS checkpoint](M1_AWS_DEVELOPMENT.md), [sanitized evidence](evidence/bootstrap-2026-11-14.json) and [release runner source](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/nix/image-contents.nix).

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

M1 remains in progress. Bootstrap readback, fixture-recipe implementation and backend allocation review are complete within their stated scopes. The remaining gates are measured selected-source build capacity within the existing host, dependency realization and fixture cross-build, the selected image's own boot/persistence/recovery results, and Rust/runtime ABI execution in that guest. No physical Mac is required for those gates.
