# M1 development without Mac hardware

Use a Linux build host and the pinned x86_64 Darwin VM as the first Burdettie reference platform. Mac hardware is not required for boot, persistence, source-owned Mach-O fixtures, shared launch/policy integration, or testing resource controls inside that VM. The production goal remains a Darwin-compatible OS running real macOS applications.

## Actual blockers

The original scratch workspace lacked Nix, QEMU, Rust/Cargo and Clang and had about 17 GiB free. Development now also uses the retained AWS Linux instance: QEMU and OVMF are installed, the official older image is verified, and a bounded bootstrap run has started. Its final result still needs retrieval after AWS sign-in. See the [AWS development checkpoint](M1_AWS_DEVELOPMENT.md). The 40-GiB host disk cannot reserve upstream's 60-GB-or-more Nix-store budget for a full image build; the smaller minimal image's peak build footprint is not specified. Selected-source build capacity and tooling remain open. [Upstream build requirements](https://puredarwin.org/handbook/getting-started.html).

The earlier requirement to identify and boot a physical Mac before closing M1 was a sequencing choice. M1 now freezes the virtual hardware and tests it first. Physical driver/firmware qualification is deferred. The exact Stickies bundle is an M4 application input, not a prerequisite for writing the loader, supervisor or security implementation.

## Primary execution path

1. Use an x86_64 Linux machine or Linux VM with Nix flakes, source/package network access and enough persistent build storage. Reserve additional space for the source tree, writable guest disks and logs beyond the Nix store budget. WSL2 provides a Linux environment on Windows and is a plausible build-host option; this specific Nix/PureDarwin/QEMU combination under WSL2 still needs its own preflight and run result.
2. Build the pinned minimal image and run the offline QEMU recipe in [ADR 0001](../adr/0001-darwin-boot-substrate.md). QEMU TCG emulates the CPU in software, so KVM or nested hardware virtualization is not required. Record tool/image hashes and exact VM configuration.
3. Prove shell access with a unique guest-command response, write a known file, stop QEMU completely, restart the same writable disk and verify the exact bytes. A kernel banner or an echoed command alone is insufficient.
4. Cross-build `fixtures/simple_cli/main.c` for x86_64 using the pinned PureDarwin SDK, `darwinCrossToolchain`, `nativeLd` and `libSystem`. First run it in the Darwin guest; then execute it through Burdettie's runtime inside that guest. Record which path ran and its output. Continue with Objective-C/Foundation and the document fixture only after the narrow ABI path is understood.
5. Implement and qualify the supervisor and selected policy controls on this virtual platform. VM isolation protects a host from a guest; it does not by itself isolate one application from other processes inside Burdettie. File, network, process and descendant restrictions still require the M5 tests.

PureDarwin constructs its SDK from source headers and supplies its cross toolchain on Linux. The existing Burdettie `scripts/build_fixtures.sh` is tied to Darwin, `xcrun` and ordinary ARM64 fixtures. Add a separate Linux/PureDarwin fixture recipe rather than treating that existing script as a hardware requirement. Follow `nix/lib/target-info.nix` at the pinned revision: its deployment target is macOS 26.5, and overriding the wrapper target can change the output architecture. This does not imply complete macOS 26.5 API support. [SDK](https://puredarwin.org/handbook/building/apple-sdk.html), [toolchains](https://puredarwin.org/handbook/building/toolchains.html), [package recipe guidance](https://puredarwin.org/handbook/porting/adding-a-package.html), [Burdettie fixture builder](https://github.com/burdettadam/burdettie-os/blob/3d9f35a700d313864c4b9ab5cce3a01435ea1c69/scripts/build_fixtures.sh).

## Smaller bootstrap option

An official prebuilt PureDarwin 20.5 minimal image is available for a smaller Linux/QEMU smoke experiment. It can establish the execution environment and serial-console automation before a full source build.

| Input | Exact value |
| --- | --- |
| Release | [PureDarwin 20.5](https://github.com/PureDarwin/PureDarwin/releases/tag/20.5) |
| Asset | `puredarwin-minimal.img` |
| Asset bytes | 274726912 |
| SHA-256 reported by GitHub | `ef65a816395680ef9210f401cebe3783c131badca1a942cb80e56bec97e717ac` |
| Release source commit | `1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290` |

Verify the downloaded bytes against that digest, use a writable copy and the release's matching firmware/runner configuration, and save only small diagnostic evidence. This older release uses a different substrate from the selected `0163b2db225b85e3bf96fc25646ea290105ac298` candidate. Its boot result cannot qualify the current candidate, Burdettie's runtime, Apple application compatibility or security enforcement. A bounded AWS bootstrap run has since started; its last observation showed kernel/device output, with the final shell/persistence result still uncollected. See the [AWS checkpoint](M1_AWS_DEVELOPMENT.md). [Release runner source](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/nix/image-contents.nix).

Standard public GitHub Ubuntu runners are a possible host for this bounded smoke experiment. Current specifications are four CPUs, 16 GB RAM and 14 GB SSD; their compute is free for public repositories. That documented disk allowance is insufficient as an assumed budget for the full source-build path. Keep any heavyweight experiment manual and separate from ordinary PR checks; do not introduce paid runners or large artifact retention implicitly. [GitHub runner specifications](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

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

Confirm the firmware paths on the host before running. The output directory must be new and its parent must exist. On a shared host, run as an unprivileged account inside a cgroup with a hard memory ceiling and CPU quota. The script additionally bounds CPU affinity, wall time, RSS and logs. Its smaller 2-vCPU/2-GiB default is a bootstrap profile, separate from the M1 reference configuration. It never downloads software, installs packages, changes its input files or opens public listeners. Unit tests validate protocol and failure handling; only an actual run can establish boot evidence.

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

M1 remains in progress. AWS now supplies the initial bootstrap execution environment. Retrieve that run's result, establish the remaining source-build capacity/tooling and record the selected candidate's own feasibility evidence.
