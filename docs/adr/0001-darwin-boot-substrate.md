# M1 platform and boot decision

Status: selected for a bounded feasibility spike. The production substrate is conditional on the evidence below. The x86_64 QEMU VM is the initial reference platform. M1 remains in progress until its build/boot/runtime experiment and contract decisions are recorded; physical Mac and real-application qualification proceed later.

## Decision

Use an x86_64 Darwin guest built from the PureDarwin XNU distribution as the first boot candidate. Pin PureDarwin at `0163b2db225b85e3bf96fc25646ea290105ac298`, observed on `upstream-xnu-12377`. Preserve its `flake.lock` and exact submodules. Begin with its minimal image, open-source `xnu-loader`, UEFI firmware and QEMU q35. Use x86_64 Linux as the reproducible build host only. Burdettie supplies its runtime, application session, launch supervisor, policy interface and selected application compatibility above the Darwin substrate.

This decision establishes where to invest the first platform experiment. It does not establish that the Burdettie runtime already runs on PureDarwin, that an arbitrary Intel Mac boots it, or that the substrate enforces Burdettie policies. Those are separate gates.

Use Linux for the complete M1 build/VM path. Existing native macOS tests can remain reference evidence, and Linux ARM64 interpreter validation remains useful; neither is a prerequisite for the x86_64 OS experiment. Prefer x86_64 application slices on that lane; additional architectures require their own qualification.

Use only the retained AWS Free plan instance for the current experiment. The provisional VM profile is reduced to two vCPUs and 2048 MiB, with single-thread TCG and the existing host CPU/memory limits. This is a derived configuration awaiting qualification on the selected source image. The upstream four-vCPU/4096-MiB configuration was not executed. No paid upgrade, new infrastructure or resource expansion is authorized by this decision.

## Why this candidate

Apple's XNU source describes the Darwin kernel and x86_64/ARM64 support, but a kernel tree alone does not supply a usable boot image, drivers, libSystem, dyld, init, filesystem or display stack. PureDarwin provides an existing integration candidate for those pieces. Its current handbook describes q35/OVMF boot, an ext4 root, launchd and an x86_64 shell. This materially reduces the initial platform integration question compared with designing a new kernel. These are upstream claims; Burdettie must reproduce the narrow checks it relies on.

The current Burdettie repository is a user-space runtime. `platform-darwin-host` wraps host services; `window-backend-darwin` depends on `winit`/`softbuffer`. The macOS target selects these backends, which does not establish that they work on bare PureDarwin. Rust standard-library/libSystem linkage and the display/input backend are explicit porting gates. Its VM demo downloads and installs a full macOS restore image through Apple's Virtualization framework on Apple Silicon. That VM remains a useful reference environment, but does not prove an independently assembled Burdettie system.

The [fixture and runtime ABI assessment](../milestones/M1_FIXTURE_ABI.md) now provides an implemented Linux cross-build recipe and source-owned C probe. The separately published draft [PR #7 source at `f6918f0`](https://github.com/burdettadam/burdettie-os/tree/f6918f029c65144eea3ca7e3b437a58bdefebb4e) makes native dialogs optional for a headless CLI and fixes immediate-exit/interpreter exit routing. [PR #5](https://github.com/burdettadam/burdettie-os/pull/5) contains separate, unmerged admission changes. These are host-tested prerequisites; selected-guest linking, runtime execution and normal-exit cleanup remain unqualified.

| Alternative | M1 disposition | Reason |
| --- | --- | --- |
| PureDarwin XNU image plus Burdettie runtime and session | Selected for feasibility | Closest existing boot integration to the Darwin product goal; requires runtime ABI, display and confinement qualification |
| Build directly from current Apple XNU source | Reserve | Adds bootstrap, drivers, userland and image assembly before the application/runtime question can be tested |
| New Rust kernel | Deferred | Rust components remain welcome; replacing kernel mechanisms adds a separate substantial research program before useful app workflows |
| Linux guest as shipping OS | Not selected by this decision | Existing portability/testing backend remains useful; changing the final substrate requires an explicit product decision |
| Full macOS VM with Burdettie session | Reference development environment | Does not satisfy independent OS boot acceptance |

## Version and source rules

Use the exact PureDarwin commit above rather than following the development branch during a run. Record the actual Nix version, lock hash, QEMU/OVMF derivations, generated image SHA-256, kernel version and any local patch hash. Pinning the branch alone is insufficient.

There is an upstream documentation mismatch. The pinned README still tells builders to supply Apple's `MacOSX11.3.sdk.tar.xz` and describes a different development branch. The pinned `nix/pkgs/toolchain/apple-sdk-pinned.nix` assembles a source-derived SDK; the current SDK handbook states that the proprietary SDK requirement has been removed. Follow the selected revision's build definitions and record any actual evaluation/build disagreement. Do not copy an Apple SDK into the public repository or treat a dependency's label as a distribution-rights review. Review the actual component closure and retain its separate notices before publishing an image.

## Initial target matrix

| Field | Selected VM experiment | Physical target |
| --- | --- | --- |
| Identifier | `puredarwin-x86_64-q35-tcg` | Unbound until hardware inventory |
| Guest CPU | x86_64; QEMU `IvyBridge,vendor=GenuineIntel` | Exact Intel CPU and model identifier required |
| Build host | x86_64 Linux with Nix flakes | Build host can be separate from target Mac |
| Firmware | Pinned OVMF X64 via upstream lock; `xnu-loader` | Actual firmware width and boot support must be recorded |
| Machine and resources | q35; single-thread TCG; 2 vCPUs; 2048 MiB RAM; derived and not yet qualified on the selected image | Actual RAM and controller/device inventory |
| Existing host bounds | 3-GiB cgroup memory ceiling; no swap; 75% of one CPU; 600 seconds per boot; 8-GiB free-disk reserve | Separately measured for a later physical test |
| Root and storage | Minimal ext4 image; GPT/ESP; AHCI raw disk | Select external test media first; actual SATA/NVMe/USB controller required |
| Display and input | Standard VGA/firmware framebuffer; QEMU xHCI keyboard and mouse; serial console | Actual GPU/framebuffer and input controllers required |
| Network | Offline for initial proof; no NIC and no SSH forwarding | Record actual Ethernet/Wi-Fi device; do not inherit upstream generic driver support |
| Execution target | x86_64 Mach-O fixture, then x86_64 application slice | Same guest/application architecture unless separately justified |
| Security claim | None from boot success; capability report starts unknown/unsupported | Actual enforcement evidence required on this backend |

The q35 machine alias is pinned indirectly by the QEMU derivation; save the expanded command and its resolved machine version with the evidence. KVM with host passthrough may be a developer acceleration lane after the TCG baseline; it is a separate machine/CPU configuration and needs its own result.

The physical-target column is a deferred hardware-support track, not an M1 closure dependency. When physical support is added, collect model identifier, CPU model/architecture/features, RAM, firmware width/version, storage/controller, GPU/display path, keyboard/trackpad/USB controllers, and network device IDs. VM evidence does not establish support for that machine.

## Bounded spike and stop conditions

Run the stages in order. A stage may consume at most two focused engineer days before producing a written pass/fail/blocker result and a decision about the next attempt. This is an investigation budget, not a delivery-date estimate. Do not continue adding frameworks while a platform gate is unresolved.

1. **Source, toolchain and capacity.** Preserve the exact tree/submodule and unchanged lock already obtained. Size individual outputs before realization; the completed four-output no-build plan does not measure temporary trees or local build outputs. Keep the existing host bounds and 8-GiB free-disk reserve. Realize the narrow fixture inputs before attempting a full minimal-image build. Record missing inputs and measured peak usage; a successful evaluation is distinct from a successful build. An unavailable or over-budget stage is `blocked`, not a failed candidate or permission to expand resources.
2. **Independent boot and persistence.** Boot a writable copy of the selected minimal image under the bounded VM profile. Capture the loader, XNU, root mount, launchd and nonce-verified shell evidence. Write a fixed test string under `/var/tmp/burdettie-m1`, flush, stop QEMU, restart the same writable image and confirm the exact bytes. Assess graceful guest shutdown separately from host termination. Boot the untouched base image separately as recovery. Save serial logs and image hashes. Neither the older 20.5 bootstrap nor a shell from the macOS IPSW VM satisfies this selected-source stage.
3. **Runtime ABI bridge.** Cross-build the implemented `simple_cli` and `m1_abi_probe` fixtures with the pinned recipe. First run the x86_64 Mach-O probes directly in the selected guest, observing allocation, exact file I/O, thread work, process status and the exit-cleanup marker. Then build/package the narrow Burdettie CLI runtime against the selected SDK/libSystem and execute the fixture through it. Record unresolved symbols, minimum-version commands, framework dependencies and actual native/guest provenance. An Apple macOS-target Rust binary is not assumed to link or run merely because both systems are Darwin. The [published exit design](https://github.com/burdettadam/burdettie-os/blob/f6918f029c65144eea3ca7e3b437a58bdefebb4e/docs/RUNTIME_EXIT_LIFECYCLE.md) requires launch-owned callbacks and draining before unmapping; an unsafe process-global drain does not close this gate.
4. **Session and enforcement boundary.** Determine the concrete display/input path for Burdettie's software renderer without assuming Apple's host AppKit. Use the reviewed [17-family backend allocation](../contracts/BACKEND_ALLOCATION.md) to assign integration and qualification work. Every family currently remains unavailable/unsupported with no evidence; an enforced launch must refuse required controls in that state. Do not map XNU presence or a successful sandbox shim to enforcement. PureDarwin's minimal console is a root shell for bringup. The Burdettie supervisor may need privileged mechanisms, but its app workers must run without supervisor authority. Permissive policy does not require running all guest apps as root. M3 completes the session boot integration; M5 proves confinement.
5. **Later physical boot probe.** This is outside M1 closure. If physical Mac support is selected later, use separate external test media and record boot selection, console, root mount, input and persistent writes. Device/installer/recovery qualification belongs to that hardware-support target; it does not block the VM preview or virtual-machine alpha.

If stages 2 or 3 cannot pass within the investigation budget, produce a gap list with estimated implementation work and decide whether to continue this candidate, use a known earlier pinned PureDarwin revision, or reassess the Darwin integration. A failed candidate does not silently authorize changing the shipping product to Linux.

## Reproducible developer commands

The following selected-image build/boot commands remain prospective. Reuse the retained exact checkout; do not obtain another source copy or replace existing test disks. Complete the per-output sizing and bounded realization decisions before running any build below. A no-build plan is not proof that the full image or QEMU/OVMF build fits. These commands do not install to a physical disk.

```sh
set -eu
cd /path/to/pinned-puredarwin
test "$(git rev-parse HEAD)" = 0163b2db225b85e3bf96fc25646ea290105ac298
git submodule status --recursive
git status --short
puredarwin_input="git+file://$(pwd)?rev=0163b2db225b85e3bf96fc25646ea290105ac298&submodules=1&shallow=1"
nix build --no-update-lock-file --out-link result-minimal "${puredarwin_input}#image-minimal"
nix build --no-update-lock-file --out-link result-qemu \
  github:NixOS/nixpkgs/fd1462031fdee08f65fd0b4c6b64e22239a77870#legacyPackages.x86_64-linux.qemu
nix build --no-update-lock-file --out-link result-ovmf \
  github:NixOS/nixpkgs/fd1462031fdee08f65fd0b4c6b64e22239a77870#legacyPackages.x86_64-linux.OVMF.fd
mkdir m1-state
cp --reflink=auto result-minimal/puredarwin-minimal.img m1-state/disk.img
chmod u+w m1-state/disk.img
cp result-ovmf/FV/OVMF_VARS.fd m1-state/OVMF_VARS.fd
chmod u+w m1-state/OVMF_VARS.fd
sha256sum result-minimal/puredarwin-minimal.img
result-qemu/bin/qemu-system-x86_64 \
  -machine q35 -accel tcg,thread=single -cpu IvyBridge,vendor=GenuineIntel \
  -smp 2 -m 2048 -vga std -display none \
  -fw_cfg name=opt/ovmf/X-PciMmio64Mb,string=2048 \
  -drive if=pflash,format=raw,unit=0,readonly=on,file=result-ovmf/FV/OVMF_CODE.fd \
  -drive if=pflash,format=raw,unit=1,file=m1-state/OVMF_VARS.fd \
  -device ich9-ahci,id=sata \
  -drive if=none,id=system,file=m1-state/disk.img,format=raw,cache=writeback \
  -device ide-hd,bus=sata.0,drive=system \
  -device qemu-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 -device usb-mouse,bus=xhci.0 \
  -nic none -serial mon:stdio
```

The QEMU/OVMF Nixpkgs revision above is the root `nixpkgs` input in the pinned PureDarwin lock. The command follows the pinned runner's boot, CPU, disk, firmware and input choices, with the guest resources reduced to the existing-host budget, network/audio omitted and the display directed to the serial terminal. Run its payload through the existing bounded host execution path, retaining the 3-GiB cgroup ceiling, no swap, 75%-of-one-CPU quota and 600-second boot deadline. The command alone does not impose those host bounds. This selected-image/tool configuration remains derived and unexecuted; the older-image bootstrap used separately recorded Ubuntu tools. Save the expanded arguments and package derivations with the result.

Use the final QEMU command for later boots; do not repeat the image or firmware copy steps over existing test state. The writable copy is essential: the upstream runner adds `snapshot=on` for a read-only Nix-store image, discarding changes at QEMU exit. The supplied upstream runner also expects `puredarwin.img` by default and includes network/SSH forwarding. The explicit command here selects the actual minimal image and an offline configuration instead.

Within the guest, use the same fixed string on both boots and compare exact output:

```sh
mkdir -p /var/tmp/burdettie-m1
printf '%s\n' burdettie-m1-persistence-v1 > /var/tmp/burdettie-m1/probe.txt
sync
cat /var/tmp/burdettie-m1/probe.txt
```

Exit QEMU completely, rerun the identical host command against the same image, then read the file again. Save an evidence record with separate `build`, `boot`, `persistence`, `runtime`, `physical_hardware` and `confinement` statuses. A pass in one field does not promote the others.

## Observed feasibility result in this session

The retained AWS host has QEMU/OVMF, the verified official 20.5 image, Nix 2.28.3 and the complete selected shallow source checkout with its exact corecrypto submodule. The lock remains unchanged at SHA-256 `00c598d3efca4bbda302fec4a43fffbfa787dc642c8c0b2c4202c65339e684d0`. The first shallow-source evaluation was blocked by the missing shallow attribute; the corrected exact-revision input explicitly carries `submodules=1&shallow=1`.

The subsequent no-build plan for `apple-sdk`, `native-ld`, `darwin-cross-toolchain` and `libsystem` passed in 232.21 seconds, with 32 planned local derivations, 264 fetched paths, 916.73 MiB download and 4811.47 MiB unpacked data. The host had 14387019776 free disk bytes at that checkpoint, with an 8-GiB reserve to preserve. Individual no-build plans have also passed; the SDK-only plan is 3 local derivations and 1908.90 MiB unpacked, and the plans overlap. The bounded SDK-only realization then passed in 75.11 seconds, with required headers/link stub checked and 12243050496 disk bytes still free. Larger linker/libSystem stages were not launched. The full-image 60-GB guideline is not a measurement of this narrower fixture lane, and the combined download estimate is not a peak-build capacity pass. See the [source-preflight evidence](../milestones/evidence/source-preflight-2026-11-14.json).

The original `bootstrap-20261113-01` failed after 600.166 seconds because the readiness matcher missed a decorated prompt followed by asynchronous console output. The interrupted next attempt is retained. The corrected `bootstrap-20261114-02` passed fresh shell nonces and exact-byte persistence across two QEMU processes: 235.948 seconds for write and 233.682 for read. The read boot replayed the ext4 journal after guest `sync` and host termination, so graceful shutdown is not qualified. The [bootstrap evidence](../milestones/evidence/bootstrap-2026-11-14.json) binds that result to the older 20.5 image and Ubuntu QEMU/OVMF variant. It does not qualify the selected source candidate.

The fixture recipe/probe and reviewed backend allocation are implemented, while selected-output realization, fixture cross-build, selected image build/boot, Rust/runtime execution and production enforcement remain unqualified. M1 remains in progress. See the [AWS checkpoint](../milestones/M1_AWS_DEVELOPMENT.md) and [development without a Mac](../milestones/M1_WITHOUT_MAC.md) for remaining gates. KVM and physical Mac ownership are not prerequisites for this TCG-based work.

## Sources

- [Burdettie README at the inspected source](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/README.md)
- [Burdettie VM guide](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/docs/VM_DEMO.md) and [actual VM launcher](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/vm-image/Sources/main.swift)
- [Burdettie Darwin host platform](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/platform-darwin-host/src/lib.rs) and [display backend dependencies](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/window-backend-darwin/Cargo.toml)
- [Current Apple XNU](https://github.com/apple-oss-distributions/xnu/blob/main/README.md)
- [Pinned PureDarwin image and runner source](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/image-contents.nix)
- [Pinned PureDarwin lock](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.lock) and [SDK recipe](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/apple-sdk-pinned.nix)
- [PureDarwin getting started](https://puredarwin.org/handbook/getting-started.html), [SDK](https://puredarwin.org/handbook/building/apple-sdk.html), [boot chain](https://puredarwin.org/handbook/images/boot-chain.html), and [hardware evidence](https://puredarwin.org/handbook/images/hardware.html)
